//! Window background media: still image, animated GIF or video.
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_SIDE: u32 = 1600;
/// Memory budget for decoded GIF frames.
const GIF_BUDGET: usize = 220 * 1024 * 1024;

pub fn is_video(path: &str) -> bool {
    let p = path.to_lowercase();
    [".mp4", ".m4v", ".mov", ".wmv", ".avi", ".mkv", ".webm", ".mpg", ".mpeg"].iter().any(|e| p.ends_with(e))
}

pub const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "m4v", "mov", "wmv", "avi", "mkv", "webm", "mpg", "mpeg"];

enum Loaded {
    Still(ColorImage),
    Frames(Vec<(ColorImage, Duration)>),
}

enum Media {
    None,
    Still(TextureHandle),
    Anim { frames: Vec<(TextureHandle, Duration)>, total: Duration, start: Instant },
    Video { tex: Option<TextureHandle>, slot: Arc<Mutex<Option<ColorImage>>>, stop: Arc<AtomicBool> },
}

pub struct Background {
    path: String,
    media: Media,
    loading: Option<Receiver<Result<Loaded, String>>>,
    pub error: String,
    /// Small RGBA sample of the current media, for building a theme from it.
    pub sample: Arc<Mutex<Option<(Vec<u8>, usize, usize)>>>,
}

impl Default for Background {
    fn default() -> Self {
        Background { path: String::new(), media: Media::None, loading: None, error: String::new(), sample: Arc::new(Mutex::new(None)) }
    }
}

fn to_color_image(img: image::RgbaImage) -> ColorImage {
    let (w, h) = img.dimensions();
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.into_raw())
}

fn shrink(img: image::DynamicImage, max: u32) -> image::RgbaImage {
    let (w, h) = (img.width(), img.height());
    let img = if w.max(h) > max { img.resize(max, max, image::imageops::FilterType::Triangle) } else { img };
    img.into_rgba8()
}

fn store_sample(sample: &Arc<Mutex<Option<(Vec<u8>, usize, usize)>>>, img: &ColorImage) {
    let [w, h] = img.size;
    let step = ((w * h) / 20000).max(1);
    let mut px = Vec::new();
    let mut n = 0;
    for (i, c) in img.pixels.iter().enumerate() {
        if i % step == 0 {
            px.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
            n += 1;
        }
    }
    *sample.lock() = Some((px, n, 1));
}

fn load(path: &str) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if path.to_lowercase().ends_with(".gif") {
        use image::AnimationDecoder;
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&bytes)).map_err(|e| e.to_string())?;
        let mut frames = Vec::new();
        let mut used = 0usize;
        let mut max = MAX_SIDE;
        for f in dec.into_frames() {
            let f = f.map_err(|e| e.to_string())?;
            let (n, d) = f.delay().numer_denom_ms();
            let delay = Duration::from_millis(((n as u64) / (d.max(1) as u64)).max(20));
            let mut img = image::DynamicImage::ImageRgba8(f.into_buffer());
            if frames.is_empty() {
                // Choose a size so that all frames fit the memory budget.
                let px = (img.width() * img.height()) as usize * 4;
                if px * 60 > GIF_BUDGET {
                    let scale = ((GIF_BUDGET / 60) as f64 / px as f64).sqrt();
                    max = ((img.width().max(img.height()) as f64 * scale) as u32).clamp(320, MAX_SIDE);
                }
            }
            img = image::DynamicImage::ImageRgba8(shrink(img, max));
            used += (img.width() * img.height()) as usize * 4;
            frames.push((to_color_image(img.into_rgba8()), delay));
            if used > GIF_BUDGET || frames.len() >= 600 {
                break;
            }
        }
        if frames.is_empty() {
            return Err("GIF has no frames".into());
        }
        if frames.len() == 1 {
            return Ok(Loaded::Still(frames.remove(0).0));
        }
        return Ok(Loaded::Frames(frames));
    }
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    Ok(Loaded::Still(to_color_image(shrink(img, MAX_SIDE))))
}

impl Background {
    pub fn stop(&mut self) {
        if let Media::Video { stop, .. } = &self.media {
            stop.store(true, Ordering::SeqCst);
        }
        self.media = Media::None;
        self.loading = None;
    }

    /// Load `path` if it changed. Empty path clears the background.
    pub fn set(&mut self, ctx: &egui::Context, path: &str) {
        if path == self.path {
            return;
        }
        self.stop();
        self.path = path.to_string();
        self.error.clear();
        *self.sample.lock() = None;
        if path.is_empty() {
            return;
        }
        if !std::path::Path::new(path).exists() {
            self.error = crate::i18n::t("Background file not found.", "Arka plan dosyası bulunamadı.").into();
            return;
        }
        if is_video(path) {
            let slot = Arc::new(Mutex::new(None));
            let stop = Arc::new(AtomicBool::new(false));
            let (p, s, st, c, sample) = (path.to_string(), slot.clone(), stop.clone(), ctx.clone(), self.sample.clone());
            std::thread::Builder::new()
                .name("makro-video".into())
                .spawn(move || {
                    if let Err(e) = video::run(&p, &s, &st, &c, &sample) {
                        *sample.lock() = None;
                        crate::send_ui(crate::UiEvent::Status(format!("{} {e}", crate::i18n::t("Video background:", "Video arka plan:"))));
                    }
                })
                .ok();
            self.media = Media::Video { tex: None, slot, stop };
            return;
        }
        let (tx, rx) = channel();
        let p = path.to_string();
        let c = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(load(&p));
            c.request_repaint();
        });
        self.loading = Some(rx);
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.loading {
            if let Ok(r) = rx.try_recv() {
                self.loading = None;
                match r {
                    Ok(Loaded::Still(img)) => {
                        store_sample(&self.sample, &img);
                        self.media = Media::Still(ctx.load_texture("bg", img, TextureOptions::LINEAR));
                    }
                    Ok(Loaded::Frames(frames)) => {
                        store_sample(&self.sample, &frames[0].0);
                        let total = frames.iter().map(|f| f.1).sum();
                        let frames = frames.into_iter().enumerate().map(|(i, (img, d))| (ctx.load_texture(format!("bg-{i}"), img, TextureOptions::LINEAR), d)).collect();
                        self.media = Media::Anim { frames, total, start: Instant::now() };
                    }
                    Err(e) => self.error = e,
                }
            }
        }
        if let Media::Video { tex, slot, .. } = &mut self.media {
            if let Some(img) = slot.lock().take() {
                match tex {
                    Some(t) => t.set(img, TextureOptions::LINEAR),
                    None => *tex = Some(ctx.load_texture("bg-video", img, TextureOptions::LINEAR)),
                }
            }
        }
    }

    /// Paint the media to cover `rect`. Returns true when something was drawn.
    pub fn paint(&mut self, ctx: &egui::Context, rect: egui::Rect) -> bool {
        self.poll(ctx);
        let tex = match &self.media {
            Media::None => return false,
            Media::Still(t) => t.clone(),
            Media::Anim { frames, total, start } => {
                let mut t = Duration::from_millis((start.elapsed().as_millis() % total.as_millis().max(1)) as u64);
                let mut pick = &frames[0];
                for f in frames {
                    if t < f.1 {
                        pick = f;
                        break;
                    }
                    t -= f.1;
                }
                ctx.request_repaint_after(Duration::from_millis(30));
                pick.0.clone()
            }
            Media::Video { tex, .. } => match tex {
                Some(t) => t.clone(),
                None => return false,
            },
        };
        let size = tex.size_vec2();
        let scale = (rect.width() / size.x).max(rect.height() / size.y);
        let shown = size * scale;
        let uv_w = (rect.width() / shown.x).min(1.0);
        let uv_h = (rect.height() / shown.y).min(1.0);
        let uv = egui::Rect::from_min_max(egui::pos2((1.0 - uv_w) / 2.0, (1.0 - uv_h) / 2.0), egui::pos2((1.0 + uv_w) / 2.0, (1.0 + uv_h) / 2.0));
        ctx.layer_painter(egui::LayerId::background()).image(tex.id(), rect, uv, Color32::WHITE);
        true
    }
}

impl Drop for Background {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Copy decoded BGRA rows into a downscaled RGBA image.
#[allow(dead_code)]
fn bgra_to_image(data: &[u8], w: usize, h: usize, stride: usize) -> ColorImage {
    let scale = ((w.max(h) as f32) / 1280.0).max(1.0);
    let (ow, oh) = (((w as f32) / scale) as usize, ((h as f32) / scale) as usize);
    let mut px = Vec::with_capacity(ow * oh);
    for y in 0..oh {
        let sy = ((y as f32 * scale) as usize).min(h - 1);
        let row = &data[sy * stride..];
        for x in 0..ow {
            let sx = ((x as f32 * scale) as usize).min(w - 1) * 4;
            px.push(Color32::from_rgb(row[sx + 2], row[sx + 1], row[sx]));
        }
    }
    ColorImage { size: [ow, oh], pixels: px }
}

#[cfg(windows)]
mod video {
    //! Video frames through Windows Media Foundation (no extra codecs bundled).
    use super::*;
    use windows::core::{HSTRING, PROPVARIANT};
    use windows::Win32::Media::MediaFoundation::*;

    pub fn run(path: &str, slot: &Arc<Mutex<Option<ColorImage>>>, stop: &AtomicBool, ctx: &egui::Context, sample: &Arc<Mutex<Option<(Vec<u8>, usize, usize)>>>) -> Result<(), String> {
        crate::uia::com_init();
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(|e| e.to_string())?;
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 1).map_err(|e| e.to_string())?;
            let attrs = attrs.ok_or("attributes")?;
            attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1).map_err(|e| e.to_string())?;
            let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path), &attrs).map_err(|e| e.to_string())?;
            let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            let mt = MFCreateMediaType().map_err(|e| e.to_string())?;
            mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|e| e.to_string())?;
            mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(|e| e.to_string())?;
            reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false).map_err(|e| e.to_string())?;
            reader.SetStreamSelection(stream, true).map_err(|e| e.to_string())?;
            reader.SetCurrentMediaType(stream, None, &mt).map_err(|e| e.to_string())?;
            let cur = reader.GetCurrentMediaType(stream).map_err(|e| e.to_string())?;
            let size = cur.GetUINT64(&MF_MT_FRAME_SIZE).map_err(|e| e.to_string())?;
            let (w, h) = ((size >> 32) as usize, (size & 0xFFFF_FFFF) as usize);
            let stride = cur.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| (s as i32).unsigned_abs() as usize).unwrap_or(w * 4).max(w * 4);
            let mut clock = Instant::now();
            let mut first = true;
            while !stop.load(Ordering::Relaxed) {
                if !crate::platform::main_window_visible() {
                    std::thread::sleep(Duration::from_millis(250));
                    clock = Instant::now();
                    continue;
                }
                let mut flags = 0u32;
                let mut ts = 0i64;
                let mut sample_obj: Option<IMFSample> = None;
                reader.ReadSample(stream, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample_obj)).map_err(|e| e.to_string())?;
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    reader.SetCurrentPosition(&windows::core::GUID::zeroed(), &PROPVARIANT::from(0i64)).map_err(|e| e.to_string())?;
                    clock = Instant::now();
                    continue;
                }
                let Some(s) = sample_obj else { continue };
                let due = Duration::from_micros((ts.max(0) / 10) as u64);
                let elapsed = clock.elapsed();
                if due > elapsed {
                    std::thread::sleep((due - elapsed).min(Duration::from_millis(200)));
                } else if elapsed > due + Duration::from_millis(250) {
                    continue; // drop late frames
                }
                let buf = s.ConvertToContiguousBuffer().map_err(|e| e.to_string())?;
                let mut ptr = std::ptr::null_mut();
                let mut len = 0u32;
                buf.Lock(&mut ptr, None, Some(&mut len)).map_err(|e| e.to_string())?;
                let data = std::slice::from_raw_parts(ptr, len as usize);
                let rows = (len as usize / stride).min(h);
                let img = bgra_to_image(data, w, rows.max(1), stride);
                let _ = buf.Unlock();
                if first {
                    first = false;
                    store_sample(sample, &img);
                }
                *slot.lock() = Some(img);
                ctx.request_repaint();
            }
            let _ = MFShutdown();
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod video {
    //! Video frames through the system ffmpeg, when installed.
    use super::*;
    use std::io::Read;

    pub fn run(path: &str, slot: &Arc<Mutex<Option<ColorImage>>>, stop: &AtomicBool, ctx: &egui::Context, sample: &Arc<Mutex<Option<(Vec<u8>, usize, usize)>>>) -> Result<(), String> {
        let (w, h) = (960usize, 540usize);
        let mut first = true;
        while !stop.load(Ordering::Relaxed) {
            let mut child = std::process::Command::new("ffmpeg")
                .args(["-loglevel", "quiet", "-re", "-i", path, "-vf", &format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"), "-r", "24", "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|_| crate::i18n::t("install ffmpeg to use video backgrounds on Linux.", "Linux'ta video arka plan için ffmpeg kurun.").to_string())?;
            let mut out = child.stdout.take().ok_or("ffmpeg")?;
            let mut frame = vec![0u8; w * h * 4];
            loop {
                if stop.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    return Ok(());
                }
                if out.read_exact(&mut frame).is_err() {
                    break;
                }
                let img = ColorImage::from_rgba_unmultiplied([w, h], &frame);
                if first {
                    first = false;
                    store_sample(sample, &img);
                }
                *slot.lock() = Some(img);
                ctx.request_repaint();
            }
            let _ = child.wait();
        }
        Ok(())
    }
}
