using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Media;
using System.Globalization;

namespace AkilliKisayol {
 public sealed class LocalizedTextBlock:TextBlock {
  bool translating;
  static LocalizedTextBlock(){TextProperty.OverrideMetadata(typeof(LocalizedTextBlock),new FrameworkPropertyMetadata("",TextChanged));}
  static void TextChanged(DependencyObject element,DependencyPropertyChangedEventArgs change){var block=(LocalizedTextBlock)element;if(block.translating||BindingOperations.IsDataBound(block,TextProperty))return;string source=change.NewValue as string;if(source==null||Ui.T(source)==source)return;try{block.translating=true;Ui.Set(block,TextProperty,source);}finally{block.translating=false;}}
 }
 public sealed class LocalizedLabel:IValueConverter {
  public object Convert(object value,Type type,object parameter,CultureInfo culture){return value is string?Ui.T((string)value):value;}
  public object ConvertBack(object value,Type type,object parameter,CultureInfo culture){throw new NotSupportedException();}
 }
 public static class Ui {
  public static string Language="en";
  public static string Normalize(string language){return language=="tr"?"tr":"en";}
  static readonly Dictionary<string,string> Turkish=new Dictionary<string,string>(StringComparer.Ordinal);
  static readonly string[] DynamicPrefixes={"Current theme: ","Local model folder: ","Target: ","Settings: "};
  sealed class Origin {public string Source,Last;}
  static readonly ConditionalWeakTable<DependencyObject,Dictionary<DependencyProperty,Origin>> origins=new ConditionalWeakTable<DependencyObject,Dictionary<DependencyProperty,Origin>>();
  static readonly DependencyProperty TemplateLanguage=DependencyProperty.RegisterAttached("TemplateLanguage",typeof(string),typeof(Ui),new PropertyMetadata(null));
  static Ui(){
   Add(@"Assign to new key|Yeni tuşa ata
Scan notices|Tarama bildirimleri
Scanning…|Taranıyor…
Scanning shortcut sources...|Kısayol kaynakları taranıyor...
{0} sources • {1} entries • {2} assignments • {3}|{0} kaynak • {1} kayıt • {2} tuş ataması • {3}
Saved Brave shortcuts and extension commands scanned|Brave kısayolları ve eklenti komutları tarandı
Customized Brave shortcut|Özelleştirilmiş Brave kısayolu
Disabled Brave shortcut|Devre dışı Brave kısayolu
Show disabled / unassigned|Devre dışı / atanmamışları göster
Application shortcuts|Uygulama kısayolları
Find an app and see its detected shortcuts and Makro+ assignments.|Bir uygulama arayın; algılanan kısayollarını ve Makro+ atamalarını görün.
Scan applications|Uygulamaları tara
Read app settings, open-window menus and Makro+ assignments in one place. Missing entries do not mean an app has no shortcuts.|Uygulama ayarları, açık pencere menüleri ve Makro+ atamaları tek yerde. Listede görünmemesi, uygulamanın kısayolu olmadığı anlamına gelmez.
Search applications|Uygulama ara
Choose an application|Bir uygulama seçin
Search actions or keys|İşlem veya tuş ara
All sources|Tüm kaynaklar
From apps|Uygulamalardan
Makro+ assignments|Makro+ atamaları
Include global shortcuts|Genel kısayolları dahil et
Coverage is partial. Defaults and conditions may differ inside the app.|Tarama kapsamı sınırlıdır. Varsayılanlar ve koşullar uygulama içinde farklı olabilir.
Some sources could not be read|Bazı kaynaklar okunamadı
Supported settings files scanned|Desteklenen ayar dosyaları tarandı
User keybindings scanned|Kullanıcı tuş atamaları tarandı
Open-window menu shortcuts read|Açık pencerenin menü kısayolları okundu
No readable app settings source found|Okunabilir uygulama ayarı kaynağı bulunamadı
Saved extension commands only; browser defaults are not scanned|Yalnızca kayıtlı eklenti komutları; tarayıcı varsayılanları taranmaz
{0} entries|{0} kayıt
Editable source|Düzenlenebilir kaynak
Read-only source|Salt okunur kaynak
Known default|Bilinen varsayılan
Reference only|Yalnızca referans
Details / edit|Ayrıntılar / düzenle
Shortcut details|Kısayol ayrıntıları
Edit this shortcut in the original application, save there, then rescan in Makro+.|Bu kısayolu kendi uygulamasında düzenleyip kaydedin, sonra Makro+ ile yeniden tarayın.
Show assigned shortcuts|Atanmış kısayolları göster
Select a shortcut settings folder|Kısayol ayarları klasörünü seçin
Close|Kapat
Details|Ayrıntılar
Shortcut name|Kısayol adı
Action to run|Çalıştırılacak işlem
Move one finger on your touchpad now; pause to finish. The recording starts after a short delay.|Touchpad’de tek parmağınızla hareket edin; bitirmek için duraklayın. Kayıt kısa bir beklemeden sonra başlar.
Record a shape, then describe its action.|Önce şekli kaydedin, sonra işlemini anlatın.
Record the shape before editing its action.|İşlemi düzenlemeden önce şekli kaydedin.
No action selected.|İşlem seçilmedi.
Ready. The shape will run after you draw it and pause.|Hazır. Şekli çizip durakladığınızda işlem çalışır.
Record a shape first.|Önce bir şekil kaydedin.
Choose an action or create one.|Bir işlem seçin veya oluşturun.
Local model memory released.|Yerel model belleği serbest bırakıldı.
Current theme: |Geçerli tema:\s
Local model folder: |Yerel model klasörü:\s
Target: |Hedef:\s
Settings: |Ayarlar:\s
Global|Genel
Makro+ assignment|Makro+ ataması
Computer shortcuts have not yet been scanned.|Bilgisayar kısayolları henüz taranmadı.
{0} entries • {1}|{0} kayıt • {1}
Left/Right Ctrl and Shift are equivalent. Left Alt is supported; Right Alt (AltGr) combinations are not triggered and are not saved. Windows virtual key identifiers are used for Turkish Q/F; physical key position is not locked. Refresh the list to see the label after a layout change. Fn hardware keys depend on the driver.\n\nActions execute once after all keys are released. If not released within 3 seconds, they are cancelled. Synthetic events do not count as triggers. Profile-specific assignments take precedence over general assignments. When communication is closed, press/release events for the matching main key are blocked; modifiers may have been sent earlier. Behavior of other keys, protected, or administrator applications cannot be guaranteed.|Sol/Sağ Ctrl ve Shift eşdeğerdir. Sol Alt desteklenir; Sağ Alt (AltGr) kombinasyonları tetiklenmez ve kaydedilmez. Türkçe Q/F için Windows sanal tuş kimlikleri kullanılır; fiziksel konum sabitlenmez. Düzen değişince etiketi yenileyin. Fn tuşları sürücüye bağlıdır.\n\nİşlemler tüm tuşlar bırakılınca bir kez çalışır. 3 saniye içinde bırakılmazsa iptal edilir. Yapay girdiler tetikleyici sayılmaz. Profile özel atamalar önceliklidir. İletim kapalıysa ana tuşun basma/bırakma olayları engellenir; değiştiriciler önceden gönderilmiş olabilir. Diğer tuşlar, korunan veya yönetici uygulamalarındaki davranış garanti edilmez.
Atomic recording and previous version keep a .bak backup. The result of the last 200 operations is saved locally. Terminal commands are saved as text; do not write passwords or access keys to commands. Administrator privileges are not required. PowerShell profiles are not loaded; cmd commands run with shell syntax. Application parameters are not passed from the shell. If the window is closed, the engine stops; you can run it in the background by minimizing to the tray.|Ayarlar atomik kaydedilir ve önceki sürüm .bak olarak saklanır. Son 200 işlem yerel olarak kaydedilir. Terminal komutları metin olarak saklanır; şifre veya erişim anahtarı yazmayın. Yönetici yetkisi gerekmez. PowerShell profilleri yüklenmez; cmd kendi kabuk sözdizimini kullanır. Pencereyi kapatmak motoru durdurur; sistem tepsisine küçülterek arka planda çalıştırabilirsiniz.
Scope, profile and behavior|Kapsam, profil ve davranış
Instant • 0 ms between steps|Anında • adımlar arasında 0 ms
Fast • 15 ms (default)|Hızlı • 15 ms (varsayılan)
Balanced • 50 ms|Dengeli • 50 ms
Compatible • 100 ms|Uyumlu • 100 ms
Slow applications • 250 ms|Yavaş uygulamalar • 250 ms
Source: Windows Precision Touchpad input|Kaynak: Windows Precision Touchpad girdisi
Source: calibrated pointer device|Kaynak: kalibre edilmiş işaretçi aygıtı
Enabled|Etkin
Disabled|Devre dışı
Key not selected|Tuş seçilmedi
Text|Metin
Wait|Bekle
Mouse click|Fare tıklaması
Mouse drag|Fare sürüklemesi
Scroll|Kaydırma
Click element|Öğeye tıkla
Click button|Düğmeye tıkla
Focus|Odakla
Open application|Uygulama aç
Open content|İçerik aç
Left|Sol
Right|Sağ
Middle|Orta
X1 — Back|X1 — Geri
X2 — Forward|X2 — İleri
Command to run (full text)|Çalıştırılacak komut (tam metin)
Spotify content link or URI|Spotify içerik bağlantısı veya URI
Target application .exe (leave blank to use previous focus)|Hedef uygulama .exe (önceki odağı kullanmak için boş bırakın)
Window title (optional)|Pencere başlığı (isteğe bağlı)
Key or mouse combination|Klavye veya fare kombinasyonu
Text, URL, item name, or command|Metin, URL, öğe adı veya komut
Wait (ms, 0–10000)|Bekleme (ms, 0–10000)
Accessible object identifier|Erişilebilir öğe kimliği
Window class (optional)|Pencere sınıfı (isteğe bağlı)
＋ Add application|＋ Uygulama ekle
＋ Choose .exe files|＋ .exe dosyaları seç
● Start recording|● Kaydı başlat
✎ Prompt shortcut|✎ Prompt ile kısayol
1. Write your task   →   2. Review the plan   →   3. Save|1. İsteğinizi yazın   →   2. Planı inceleyin   →   3. Kaydedin
Accept software-remapped keyboard and mouse inputs from device utilities|Cihaz yazılımlarının yeniden atadığı klavye ve fare girdilerini kabul et
Add / update preference|Tercih ekle / güncelle
Add a shape|Şekil ekle
Add application|Uygulama ekle
Add assignments|Atamaları ekle
Add folder|Klasör ekle
Add profile|Profil ekle
Add selected|Seçileni ekle
Add shortcut for launching the application|Uygulamayı açmak için kısayol ekle
Add shortcut for the application|Uygulamaya özel kısayol ekle
Add source file|Kaynak dosyası ekle
Add step|Adım ekle
Advanced item matching|Gelişmiş öğe eşleştirme
After releasing the shortcut, it is sent to the same window in the foreground. It may not work in management applications.|Kısayolu bıraktığınızda ön plandaki aynı pencereye gönderilir. Yönetici yetkili uygulamalarda çalışmayabilir.
AI model • Makro+|Yapay zekâ modeli • Makro+
AI model settings|Yapay zekâ modeli ayarları
API base URL (include /v1 when required)|API temel adresi (gerekiyorsa /v1 ekleyin)
API key (optional, encrypted for your Windows account)|API anahtarı (isteğe bağlı, Windows hesabınıza özel şifrelenir)
Application preferences|Uygulama tercihleri
Applications do not need to be open. Choose their .exe files, or select from running applications below. Registered applications stay available when closed.|Uygulamaların açık olması gerekmez. .exe dosyalarını veya aşağıdaki çalışan uygulamaları seçin. Kayıtlı uygulamalar kapalıyken de kullanılabilir.
Applies immediately to all open Makro+ windows and is saved for the next launch.|Açık Makro+ pencerelerine hemen uygulanır ve sonraki açılış için kaydedilir.
Apply step|Adımı uygula
Assignments profile|Atama profili
Assignments, macros, profiles, and application preferences are exported. The operation history is not exported. You can match application paths and review assignments on another computer.|Atamalar, makrolar, profiller ve uygulama tercihleri aktarılır. İşlem geçmişi aktarılmaz. Başka bir bilgisayarda uygulama yollarını eşleştirip atamaları inceleyebilirsiniz.
Bring your settings with you|Ayarlarınızı yanınızda taşıyın
Browse GGUF…|GGUF seç…
Change groups by selecting the profile in the bottom left. Assignments across all profiles are shared; profile-specific assignments take precedence within the same key and scope. External source files are not changed during profile switching.|Sol alttaki profil seçimiyle geçiş yapın. Tüm profillere ait atamalar paylaşılır; aynı tuş ve kapsamda profile özel atamalar önceliklidir. Profil değişince dış kaynak dosyaları değiştirilmez.
Change model|Modeli değiştir
Check again|Yeniden denetle
Check current system compatibility|Mevcut sistem uyumluluğunu denetle
Choose .exe|.exe seç
Choose a local GGUF or connect to a chat-completions server. No model is bundled or downloaded automatically.|Yerel GGUF seçin veya chat-completions sunucusuna bağlanın. Model pakete dahil edilmez, otomatik indirilmez.
Choose your atmosphere|Görünümünüzü seçin
Close on key release • Resource tags separate file records and defaults|Tuş bırakıldığında çalışır • Kaynak etiketleri dosya kayıtları ve varsayılanları ayırır
CONFLICT: same key and overlapping scope. PRIORITY: personal application scope takes precedence over general assignment. POSSIBLE: default, condition, or array start match. Windows does not list all hooks active when external applications run.|ÇAKIŞMA: aynı tuş ve örtüşen kapsam. ÖNCELİK: uygulamaya özel atama genel atamadan önce çalışır. OLASI: varsayılan, koşul veya dizi başlangıcı eşleşmesi. Windows, diğer uygulamaların tüm etkin girdilerini listelemez.
Connection converted to Spotify URI and content opens. Direct playback is not supported; playback is not verified. Spotify must be installed; network may be required for content.|Bağlantı Spotify URI biçimine çevrilir ve içerik açılır. Doğrudan oynatma desteklenmez, oynatma doğrulanmaz. Spotify kurulu olmalıdır; içerik için ağ gerekebilir.
Continuously recognize touchpad shapes|Touchpad şekillerini sürekli algıla
Copy downloaded .gguf files into models next to MakroPlus.exe. Use Configure model → Open models folder. One file is detected automatically; select a file when there are several.|İndirdiğiniz .gguf dosyalarını MakroPlus.exe yanındaki models klasörüne kopyalayın. Modeli ayarla → Model klasörünü aç yolunu kullanın. Tek dosya otomatik algılanır; birden fazla varsa seçim yapın.
Create / edit action|İşlem oluştur / düzenle
Create assignment with Prompt|Prompt ile atama oluştur
Create theme|Tema oluştur
Create touchpad shortcut with prompt|Prompt ile touchpad kısayolu oluştur
Create with prompt|Prompt ile oluştur
Dark theme|Koyu tema
Describe action with prompt|İşlemi prompt ile anlat
Describe your shortcut|Kısayolunuzu anlatın
Detect my touchpad|Touchpad’imi algıla
Device and application diagnostics|Cihaz ve uygulama tanılaması
Disable general write keys without modifiers|Değiştirici içermeyen genel yazı tuşlarını devre dışı bırak
Down|Aşağı
Draw a saved one-finger shape and pause. No hotkey, mouse button or activation tap is needed. Only confident matches run, in their assigned profile and application scope.|Kayıtlı tek parmaklı şekli çizip duraklayın. Tuş, fare düğmesi veya etkinleştirme dokunuşu gerekmez. Yalnızca yeterli eşleşmeler kendi profil ve kapsamlarında çalışır.
Draw once. Run it with your touchpad.|Bir kez çizin. Touchpad’inizle çalıştırın.
Duplicate|Çoğalt
Edit Macro Step|Makro adımını düzenle
Edit shape / action|Şekli / işlemi düzenle
Edit shortcut source|Kısayol kaynağını düzenle
Edit step|Adımı düzenle
Edit your step as you like|Adımınızı istediğiniz gibi düzenleyin
Enable when importing enabled and conflicting assignments|İçe aktarılan etkin ve çakışan atamaları etkinleştir
Example: Ctrl+Alt+K or F9. If left blank, only the key binding is removed; the shortcut file is preserved.|Örnek: Ctrl+Alt+K veya F9. Boş bırakılırsa yalnızca tuş ataması kaldırılır; kısayol dosyası korunur.
Example: ctrl+alt+k. Sequential series: ctrl+k ctrl+c.|Örnek: ctrl+alt+k. Ardışık dizi: ctrl+k ctrl+c.
Existing assignments are preserved. Match missing application paths with .exe files on this computer. Invalid or conflicting assignments are disabled; you can edit them later.|Mevcut atamalar korunur. Eksik uygulama yollarını bu bilgisayardaki .exe dosyalarıyla eşleştirin. Geçersiz veya çakışan atamalar devre dışı bırakılır; daha sonra düzenleyebilirsiniz.
Export settings|Ayarları dışa aktar
Fast planning (disable optional reasoning for local models)|Hızlı planlama (yerel modellerin isteğe bağlı akıl yürütmesini kapat)
Finish recording (F10)|Kaydı bitir (F10)
For a legacy driver, click Detect my touchpad and move only your touchpad. Calibration selects the Windows pointer source; the app cannot identify vendor-only inputs that Windows does not expose. Normal cursor movement continues while listening. Switch to a target application to try a saved shape; recognition is suspended in Makro+ itself.|Eski sürücülerde Touchpad’imi algıla seçeneğine basıp yalnızca touchpad’i hareket ettirin. Kalibrasyon Windows işaretçi kaynağını seçer; üreticiye özel gizli girdiler algılanamaz. Dinlerken imleç hareketi sürer. Şekli başka bir uygulamada deneyin; Makro+ içinde algılama duraklatılır.
General scope applies to all applications. Select Profile Independently; select All Profiles to have it work in all profiles.|Genel kapsam tüm uygulamalarda geçerlidir. Profili ayrıca seçin; her profilde çalışması için Tüm profiller’i seçin.
Generate preview|Önizleme oluştur
Import Macro+ • Settings|Makro+ • Ayarları içe aktar
Import settings|Ayarları içe aktar
Import theme|Tema içe aktar
Increase this budget if a reasoning model runs out of tokens before returning a final plan. Smaller models and shorter plans are usually faster.|Akıl yürüten model planı tamamlamadan token sınırına ulaşırsa bütçeyi artırın. Küçük modeller ve kısa planlar genellikle daha hızlıdır.
Key|Tuş
Key (you can also type it in the prompt)|Tuş (prompt içinde de yazabilirsiniz)
Key combination|Tuş kombinasyonu
KEYBOARD  •  MOUSE  •  TOUCHPAD|KLAVYE  •  FARE  •  TOUCHPAD
Left-clicks on items that can be found are played over the item. Other clicks, drags, and scrolls use the window-relative position; if the window size changes, the macro stops. Keyboard recording saves physical keys and AltGr information; use the same keyboard layout. Target applications must be open during playback. Escape cancels the remaining steps.|Bulunabilen öğelere tıklama doğrudan öğe üzerinde tekrarlanır. Diğer tıklama, sürükleme ve kaydırmalar pencereye göre konum kullanır; pencere boyutu değişirse makro durur. Klavye kaydı fiziksel tuşları ve AltGr bilgisini saklar; aynı klavye düzenini kullanın. Çalıştırırken hedef uygulamalar açık olmalıdır. Escape kalan adımları iptal eder.
Let's clarify a few details|Birkaç ayrıntıyı netleştirelim
LIVE PREVIEW|CANLI ÖNİZLEME
Listening…|Dinleniyor…
Listening… Escape to cancel|Dinleniyor… İptal için Escape
Local LLM can create open, connect, focus, key, text, button, and command steps for all registered applications. No restrictions are applied based on application name. You can preview all generated steps, including commands. The model does not see the application's screen; the application's result is not additionally verified. Escape cancels the remaining steps of the working sequence; sent steps cannot be retrieved. You can also select the rule interpreter while the model is closed.|Yerel model kayıtlı tüm uygulamalar için açma, bağlantı, odaklama, tuş, metin, düğme ve komut adımları oluşturabilir. Uygulama adına göre kısıtlama uygulanmaz. Komutlar dahil tüm adımları önizleyebilirsiniz. Model ekranı görmez; uygulama sonucu ayrıca doğrulanmaz. Escape kalan adımları iptal eder; gönderilen adımlar geri alınamaz. Model kapalıyken kural yorumlayıcıyı da seçebilirsiniz.
Locations are aligned to the top-left of the window. Proportional scaling helps in different sizes; check locations in the changed layout.|Konumlar pencerenin sol üstüne göredir. Orantılı ölçekleme farklı boyutlarda yardımcı olur; değişen düzende konumları kontrol edin.
Macro recorder|Makro kaydedici
Make it your own|Kendinize göre düzenleyin
Maximum output tokens|En fazla çıktı token’ı
Model file|Model dosyası
Model folder — next to MakroPlus.exe|Model klasörü — MakroPlus.exe yanında
Model ID|Model kimliği
Model request could not be completed|Model isteği tamamlanamadı
Mouse button|Fare tuşu
Mouse buttons & wheel|Fare tuşları ve tekerlek
No activation key or button is required. Record a distinct one-finger shape, then pause. A saved shape can run in the background when continuous recognition is enabled.|Etkinleştirme tuşu gerekmez. Belirgin bir şekli tek parmakla çizip duraklayın. Sürekli algılama etkinken kayıtlı şekil arka planda çalışabilir.
No applications added yet. Add from open applications or .exe files.|Henüz uygulama eklenmedi. Çalışan uygulamalardan veya .exe dosyalarından ekleyin.
No conflicts found among the read recordings.|Okunan kayıtlar arasında çakışma bulunamadı.
No operations yet. Results will appear here when a shortcut is triggered.|Henüz işlem yok. Kısayol çalıştırıldığında sonuçlar burada görünür.
No registered shortcut found to display for this application.|Bu uygulama için gösterilecek kayıtlı kısayol bulunamadı.
No shapes saved yet. Detect your touchpad, add a shape and select its action.|Henüz şekil yok. Touchpad’inizi algılayın, bir şekil ekleyip işlemini seçin.
Open models folder|Model klasörünü aç
Pause to finish (ms)  |Bitirme beklemesi (ms)\s\s
  Match confidence  |  Eşleşme güveni\s\s
Pick a preset, create your own palette, or import a theme shared by someone else.|Hazır tema seçin, kendi paletinizi oluşturun veya paylaşılan bir temayı içe aktarın.
Pick colors or type #RRGGBB values. The preview updates as you edit.|Renk seçin veya #RRGGBB değeri yazın. Düzenlerken önizleme güncellenir.
Press and hold the Right Ctrl button for about half a second. The Help panel will appear without focus; it closes when you release the button. If you press another key, the panel closes and normal Ctrl combinations work.|Sağ Ctrl tuşunu yaklaşık yarım saniye basılı tutun. Rehber odağı almadan görünür ve tuşu bırakınca kapanır. Başka tuşa basarsanız rehber kapanır ve normal Ctrl kombinasyonları çalışır.
Press key|Tuşa bas
Preview will appear here. No step will run without saving.|Önizleme burada görünür. Kaydetmeden hiçbir adım çalışmaz.
Primary action|Birincil işlem
Processing… Cancel|İşleniyor… İptal et
Profile name|Profil adı
Put downloaded .gguf files here; no installation is needed. One model is detected automatically. With several models, use Browse GGUF to choose. Other folders are also supported.|İndirilen .gguf dosyalarını buraya koyun; kurulum gerekmez. Tek model otomatik algılanır. Birden fazla model varsa GGUF seç düğmesini kullanın. Başka klasörler de desteklenir.
Quick Applications|Uygulamalara hızlı erişim
Quickly add your applications|Uygulamalarınızı hızlıca ekleyin
Record shape|Şekli kaydet
Refresh application list|Uygulama listesini yenile
Refresh models folder|Model klasörünü yenile
Refresh open applications|Çalışan uygulamaları yenile
Registered apps and selected .exe files can be closed while creating this shortcut. Use Global scope to launch an app from anywhere. UI actions need their target window when the shortcut runs.|Kısayol oluştururken kayıtlı uygulamalar kapalı olabilir. Her yerden uygulama açmak için Genel kapsamı seçin. Arayüz işlemleri çalıştırılırken hedef pencere gerekir.
Remember application preferences from saved connection assignments|Kayıtlı bağlantı atamalarından uygulama tercihlerini hatırla
Remember your selection|Seçiminizi hatırla
Remove from clipboard|Listeden kaldır
Remove selected additional source|Seçili ek kaynağı kaldır
Remove selected option|Seçili seçeneği kaldır
Remove selected step|Seçili adımı kaldır
Request structured JSON output|Yapılandırılmış JSON çıktı iste
Resize window position proportionally when size changes|Boyut değiştiğinde konumları orantılı ölçekle
Rules preserve other fields like args. A command starting with '-' is a default removal rule; an empty command blocks the operation for this key.|Kurallar args gibi diğer alanları korur. '-' ile başlayan komut varsayılanı kaldırır; boş komut bu tuşun işlemini engeller.
Save and apply|Kaydet ve uygula
Save help settings|Rehber ayarlarını kaydet
Save macro|Makroyu kaydet
Save model settings|Model ayarlarını kaydet
Save preferences|Tercihleri kaydet
Save profiles|Profilleri kaydet
Save report|Raporu kaydet
Save settings|Ayarları kaydet
Save shape shortcut|Şekil kısayolunu kaydet
Save the combination to send|Gönderilecek kombinasyonu yakala
Save to source|Kaynağa kaydet
Saved plans execute without calling the model. Explicit Wait steps and recorded pauses are preserved. If an app misses rapid key sequences, choose a slower speed.|Kayıtlı planlar modele başvurmadan çalışır. Bekleme adımları ve kayıtlı duraklamalar korunur. Uygulama hızlı tuş dizilerini kaçırıyorsa daha yavaş hız seçin.
Scan sources|Kaynakları tara
Scroll horizontally|Yatay kaydır
Select application|Uygulama seç
Select applications to add|Eklenecek uygulamaları seçin
Select shortcut source|Kısayol kaynağını seç
Select target .exe|Hedef .exe seç
Select the application, create a shortcut for it.|Uygulamayı seçin, ona bir kısayol oluşturun.
Shortcut|Kısayol
Shortcut active|Kısayol etkin
Shortcut for the application|Uygulamaya özel kısayol
Shortcut profiles|Kısayol profilleri
Shortcut with Prompt|Prompt ile kısayol
Show all|Tümünü göster
Show example panel (6 seconds)|Örnek paneli göster (6 saniye)
Show operation once|İşlemi bir kez göster
Show shortcut guide when holding the guide key|Rehber tuşu basılıyken kısayol rehberini göster
Start recording, switch to the target application, and perform your operation. F10 ends the recording. This application's own windows and synthetic inputs are not recorded. Personal shortcuts do not work during recording.|Kaydı başlatın, hedef uygulamaya geçin ve işleminizi yapın. F10 kaydı bitirir. Makro+ pencereleri ve yapay girdiler kaydedilmez. Kayıt sırasında kişisel kısayollar çalışmaz.
Stop capture|Yakalamayı durdur
Supports OpenAI-compatible chat/completions APIs, including local model servers. Prompts and registered application context are sent to the server you choose.|Yerel sunucular dahil OpenAI uyumlu chat/completions API’lerini destekler. Prompt ve kayıtlı uygulama bağlamı seçtiğiniz sunucuya gönderilir.
Target app & advanced options|Hedef uygulama ve gelişmiş seçenekler
Target application — connection or operation which application should open?|Hedef uygulama — bağlantı veya işlem hangi uygulamada açılsın?
Task, game and work schedule|İş, oyun ve çalışma düzeni
The Desktop, Start menu, and application user settings are scanned. A source record is not proof that the button is currently being captured by the application. Select and edit an entry from the list.|Masaüstü, Başlat menüsü ve uygulama kullanıcı ayarları taranır. Kaynak kaydı, tuşun şu anda yakalandığını kanıtlamaz. Listeden bir kayıt seçip düzenleyin.
The shortcuts section in the computer reads .lnk files and VS Code-compatible user shortcut files and edits them in the sources. You can add additional files/folders to the scan. The previous version source file is preserved alongside .akilli-kisayol.bak. New key assignments are provided for Windows defaults. There is no common Windows API that provides all shortcuts from other applications.|Bilgisayardaki kısayollar bölümü .lnk ve VS Code uyumlu kullanıcı kısayol dosyalarını okuyup kaynaklarında düzenler. Taramaya dosya veya klasör ekleyebilirsiniz. Önceki sürüm .akilli-kisayol.bak olarak saklanır. Windows varsayılanlarına yeni tuş atanabilir. Diğer uygulamaların tüm kısayollarını sunan ortak Windows API’si yoktur.
Theme designer • Makro+|Tema tasarımcısı • Makro+
Theme name|Tema adı
These preferences are added as defaults to subsequent prompts. The application you typed in the prompt or the target you selected always comes first.|Bu tercihler sonraki promptlarda varsayılan olur. Promptta yazdığınız uygulama veya seçtiğiniz hedef her zaman önceliklidir.
This text is a command and operates with your user privileges. Do not write passwords/keys. PowerShell code is passed as a separate argument in UTF-16 Base64; cmd interprets its own shell syntax. Output is not captured; results and exit code are logged to history.|Bu komut kullanıcı yetkilerinizle çalışır. Şifre veya anahtar yazmayın. PowerShell kodu UTF-16 Base64 bağımsız değişkeniyle iletilir; cmd kendi sözdizimini yorumlar. Çıktı yakalanmaz; sonuç ve çıkış kodu geçmişe yazılır.
This version opens Spotify URIs. It does not verify whether playback has started or a track is playing. There is no direct playback/OAuth integration. Web and Spotify content may require a network connection; basic shortcut management is offline.|Spotify URI’leri açılır. Oynatmanın başladığı veya parça çaldığı doğrulanmaz. Doğrudan oynatma/OAuth entegrasyonu yoktur. Web ve Spotify içeriği ağ gerektirebilir; temel kısayol yönetimi çevrimdışıdır.
Touchpad action with prompt|Prompt ile touchpad işlemi
Touchpad shapes|Touchpad şekilleri
Touchpad shapes, always ready|Touchpad şekilleri, her zaman hazır
Trigger, target app & advanced options|Tetikleyici, hedef uygulama ve gelişmiş seçenekler
Unsupported response_format, temperature and token-count parameters are adapted automatically when the server reports them. You can also turn structured output off. Plans are validated before saving; model quality varies.|Sunucu desteklenmediğini bildirdiğinde response_format, temperature ve token parametreleri otomatik uyarlanır. Yapılandırılmış çıktıyı da kapatabilirsiniz. Planlar kaydetmeden önce doğrulanır; model kalitesi değişebilir.
Use GPU for local models|Yerel modeller için GPU kullan
Use the touchpad source selected on the Touchpad gestures page. Record a larger shape if the trace is too short.|Touchpad şekilleri sayfasında seçilen kaynağı kullanın. İz kısa kalıyorsa daha büyük bir şekil çizin.
Use this action|Bu işlemi kullan
Use Windows Precision Touchpad|Windows Precision Touchpad kullan
Use your own model|Kendi modelinizi kullanın
Uses the model's embedded chat template and the bundled llama.cpp runtime. GGUF architecture support and available memory depend on the runtime and your hardware.|Modelin gömülü sohbet şablonunu ve paketteki llama.cpp çalışma ortamını kullanır. GGUF mimarisi desteği ve kullanılabilir bellek çalışma ortamına ve donanımınıza bağlıdır.
View shortcuts of the open application|Açık uygulamanın kısayollarını göster
When enabled, remapped input from other software can also trigger shortcuts. Makro+ playback is always excluded. Firmware-only Fn keys and vendor buttons must be exposed by the device driver as Windows keys or mouse buttons.|Etkinse diğer yazılımların yeniden atadığı girdiler de kısayol çalıştırabilir. Makro+ tarafından üretilen girdiler hariçtir. Fn ve üretici tuşlarının sürücü tarafından Windows tuşu veya fare düğmesi olarak sunulması gerekir.
When transmission is closed, the main key is blocked; Ctrl/Alt/Shift events may reach the source application. Full control over external applications is not guaranteed.|İletim kapalıysa ana tuş engellenir; Ctrl/Alt/Shift kaynak uygulamaya ulaşabilir. Diğer uygulamalar üzerinde tam denetim garanti edilmez.
Where should the shortcut run?|Kısayol nerede çalışsın?
Windows 10/11, 64-bit, and .NET Framework 4.8 are targeted. Choose your own GGUF or a compatible model server in AI model. No model is bundled. Mouse assignments in installed applications and device drivers may differ on other computers.|Windows 10/11, 64 bit ve .NET Framework 4.8 hedeflenir. Kendi GGUF modelinizi veya uyumlu sunucuyu Yapay zekâ modeli bölümünden seçin. Paket model içermez. Uygulama ve sürücü fare atamaları diğer bilgisayarlarda farklı olabilir.
Write the assignment I want|İstediğim atamayı yaz
Your request|İsteğiniz
Your shortcuts. Your colors.|Sizin kısayollarınız. Sizin renkleriniz.
ACTIVE PROFILE|ETKİN PROFİL
Work|İş
Game|Oyun
Study|Çalışma
Key not selected|Tuş seçilmedi
Press and hold Del, then press 5. Escape cancels; actions do not work during recording.|Del tuşunu basılı tutup 5’e basın. Escape iptal eder; kayıt sırasında atamalar çalışmaz.
Application-specific assignment takes precedence over general assignments on the same key.|Aynı tuşta uygulamaya özel atama, genel atamadan önceliklidir.
Home|Ana sayfa
All shortcuts|Tüm kısayollar
My shortcuts|Kısayollarım
Recommended shortcuts|Önerilen kısayollar
System actions|Sistem işlemleri
Keyboard shortcuts|Klavye kısayolları
Mouse shortcuts|Fare kısayolları
Touchpad gestures|Touchpad şekilleri
Applications|Uygulamalar
AI model|Yapay zekâ modeli
Profiles|Profiller
Preferences|Tercihler
Shortcut guide|Kısayol rehberi
Computer shortcuts|Bilgisayardaki kısayollar
Conflicts|Çakışmalar
History|Geçmiş
Transfer and diagnostics|Aktarım ve tanılama
Appearance|Görünüm
Settings|Ayarlar
OVERVIEW|GENEL BAKIŞ
INPUTS|GİRDİLER
WORKSPACE|ÇALIŞMA ALANI
More tools|Diğer araçlar
TOOLS & SETTINGS|ARAÇLAR VE AYARLAR
Make it yours.|Size göre çalışsın.
Language|Dil
Interface language|Arayüz dili
English|English
Everyday|Günlük kullanım
System|Sistem
Power|Güç ve oturum
All|Tümü
Ready|Hazır
Paused|Duraklatıldı
Active shortcuts|Etkin kısayol
Current profile|Geçerli profil
Direct shortcut execution|Kısayollar doğrudan çalışır
Custom model server|Özel model sunucusu
▶ Resume shortcuts|▶ Kısayolları sürdür
Ⅱ Pause shortcuts|Ⅱ Kısayolları duraklat
Your next action, one shortcut away.|Bir sonraki işiniz tek kısayol uzakta.
Create once. Use it anywhere.|Bir kez oluşturun. Her yerde kullanın.
Keys, mouse buttons and touchpad shapes — all in one place.|Klavye, fare tuşları ve touchpad şekilleri tek bir yerde.
New shortcut|Yeni kısayol
Create with Prompt|Prompt ile oluştur
Record macro|Makro kaydet
Add shortcut|Kısayol ekle
＋ Add shortcut|＋ Kısayol ekle
Explore recommendations|Önerileri keşfet
Build your own|Kendi kısayolunu oluştur
Choose an input, then choose what it does.|Bir girdi seçin ve ne yapacağını belirleyin.
Describe it|İstediğini yaz
Write a prompt and review the generated plan.|Prompt yazın ve oluşturulan planı inceleyin.
Record it|Hareketlerini kaydet
Capture keys, clicks and movement as a macro.|Tuşları, tıklamaları ve hareketleri makro olarak kaydedin.
Choose it|Doğrudan ata
Pick an action without a model or an open application.|Model veya açık uygulama gerektirmeden bir işlem seçin.
Start with a useful idea|İşe yarayan bir fikirle başlayın
Browse ready-to-customize actions. Nothing is added automatically.|Düzenleyebileceğiniz hazır işlemleri inceleyin. Hiçbiri otomatik eklenmez.
Browse all recommendations|Tüm önerilere göz at
Review an action, choose your input, then save it.|İşlemi inceleyin, girdinizi seçin ve kaydedin.
Keyboard, mouse or touchpad|Klavye, fare veya touchpad
Choose a key / mouse button|Klavye veya fare tuşu seç
Draw a touchpad shape|Touchpad şekli çiz
Add…|Ekle…
No trigger is assigned until you choose one. Existing conflicts are checked before saving.|Siz seçene kadar tetikleyici atanmaz. Kaydetmeden önce mevcut çakışmalar denetlenir.
Requires confirmation when triggered|Çalıştırılırken onay istenir
Runs directly when triggered|Tetiklendiğinde doğrudan çalışır
No model required|Model gerekmez
Manage Windows without keeping an application open.|Bir uygulamayı açık tutmadan Windows işlemlerini yönetin.
Check Windows updates|Windows güncellemelerini denetle
Start an online scan now. Results appear here and in history; scanning can take several minutes. Nothing is installed.|Çevrimiçi tarama başlatır. Sonuç burada ve geçmişte gösterilir; tarama birkaç dakika sürebilir. Güncelleme yüklemez.
Lock computer|Bilgisayarı kilitle
Lock your session immediately.|Oturumunuzu hemen kilitler.
Show desktop|Masaüstünü göster
Hide or restore your open windows.|Açık pencerelerinizi gizler veya geri getirir.
Switch workspace|Çalışma alanını değiştir
See open windows and virtual desktops.|Açık pencereleri ve sanal masaüstlerini gösterir.
Capture a screen region|Ekranın bir bölümünü yakala
Choose an area and copy a screenshot.|Bir alan seçip ekran görüntüsünü kopyalayın.
Clipboard history|Pano geçmişi
Choose from copied items. Enable clipboard history in Windows first.|Kopyalanan öğelerden seçim yapın. Önce Windows pano geçmişini etkinleştirin.
Emoji and symbols|Emoji ve simgeler
Insert symbols, emoji and special characters.|Simge, emoji ve özel karakter ekleyin.
Open File Explorer|Dosya Gezgini’ni aç
Jump straight to your files.|Dosyalarınıza doğrudan ulaşın.
Open Task Manager|Görev Yöneticisi’ni aç
Inspect running applications and resource usage.|Çalışan uygulamaları ve kaynak kullanımını inceleyin.
Sound output settings|Ses çıkışı ayarları
Change speakers, headphones and input devices.|Hoparlör, kulaklık ve giriş aygıtlarını değiştirin.
Network settings|Ağ ayarları
Manage Wi-Fi, Ethernet and connection settings.|Wi-Fi, Ethernet ve bağlantı ayarlarını yönetin.
Storage settings|Depolama ayarları
Review disk usage and Windows cleanup options.|Disk kullanımını ve Windows temizleme seçeneklerini inceleyin.
Open Windows Update|Windows Update’i aç
View update status and available Windows update controls.|Güncelleme durumunu ve Windows güncelleme seçeneklerini görüntüleyin.
Close open applications|Açık uygulamaları kapat
Request closure of visible application windows in your session. Save dialogs are respected; background services and Makro+ stay open.|Oturumunuzdaki görünür uygulama pencerelerine kapatma isteği gönderir. Kayıt uyarıları korunur; arka plan servisleri ve Makro+ açık kalır.
Shut down computer|Bilgisayarı kapat
Ask Windows to shut down after confirmation. Applications may block shutdown to save your work.|Onayınızdan sonra Windows’a kapatma isteği gönderir. Uygulamalar çalışmanızı kaydetmek için kapatmayı engelleyebilir.
Restart computer|Bilgisayarı yeniden başlat
Ask Windows to restart after confirmation. No forced termination.|Onayınızdan sonra yeniden başlatma isteği gönderir. Uygulamaları zorla sonlandırmaz.
Confirm system action|Sistem işlemini onayla
Close these application windows?|Bu uygulama pencereleri kapatılsın mı?
Cancelled. No action was sent.|İptal edildi. İşlem gönderilmedi.
Request sent to Windows.|İstek Windows’a gönderildi.
Checking Windows updates… You can keep using Makro+.|Windows güncellemeleri denetleniyor… Makro+ kullanmaya devam edebilirsiniz.
An update scan is already running.|Güncelleme taraması zaten sürüyor.
A close request is already in progress.|Kapatma isteği zaten işleniyor.
A power request is already in progress.|Güç işlemi zaten işleniyor.
No eligible application windows are open.|Kapatılabilecek uygulama penceresi yok.
Close requests sent:|Gönderilen kapatma isteği:
Save prompts may keep applications open.|Kayıt uyarıları uygulamaları açık tutabilir.
Available software updates:|Bulunan yazılım güncellemesi:
Update scan partially completed:|Güncelleme taraması kısmen tamamlandı:
Open Windows Update to review details. Nothing was installed.|Ayrıntılar için Windows Update’i açın. Hiçbir güncelleme yüklenmedi.
Windows could not complete the update scan.|Windows güncelleme taramasını tamamlayamadı.
Windows Update scan failed. Check your connection and Windows Update policy.|Windows Update taraması başarısız. Bağlantınızı ve Windows Update ilkesini denetleyin.
Choose a valid system action.|Geçerli bir sistem işlemi seçin.
Action type|İşlem türü
System action|Sistem işlemi
Assignment name|Kısayol adı
Trigger combination|Tetikleyici kombinasyonu
Profile|Profil
Scope|Kapsam
Global — in every application|Genel — tüm uygulamalarda
Application-specific|Uygulamaya özel
Application|Uygulama
File / folder|Dosya / klasör
Web address|Web adresi
URI / deep link|URI / uygulama bağlantısı
Send key|Tuş gönder
Media|Medya
Edit shortcut|Kısayolu düzenle
One key, one action. You define the scope.|Bir girdi, bir işlem. Nerede çalışacağını siz seçin.
Save assignment|Kısayolu kaydet
Cancel|İptal
Clear|Temizle
● Capture keyboard / mouse|● Klavye / fare girdisini yakala
Mouse button or wheel selection|Fare tuşu veya tekerlek seçimi
Shortcut enabled|Kısayol etkin
Also send the main key to the source application|Ana tuşu kaynak uygulamaya da gönder
All profiles|Tüm profiller
Select a registered application or choose its .exe. The application can be closed.|Kayıtlı bir uygulama veya .exe dosyası seçin. Uygulama kapalı olabilir.
Choose an .exe file|.exe dosyası seç
Quick select from added applications|Eklenen uygulamalardan hızlı seçim
Keys to send|Gönderilecek tuşlar
Media action|Medya işlemi
Address|Adres
Executable file|Çalıştırılabilir dosya
File or folder path|Dosya veya klasör yolu
Startup parameters (optional)|Başlatma parametreleri (isteğe bağlı)
Select file|Dosya seç
Select folder|Klasör seç
Play / pause|Oynat / duraklat
Next track|Sonraki parça
Previous track|Önceki parça
Volume up|Sesi yükselt
Volume down|Sesi azalt
Mute / unmute|Sesi kapat / aç
Search|Ara
Status|Durum
Combination|Kombinasyon
Action|İşlem
Source|Kaynak
Details|Ayrıntılar
Edit|Düzenle
Enable / disable|Etkinleştir / devre dışı bırak
Delete|Sil
Rescan|Yeniden tara
Restore backup|Yedeği geri yükle
Show source|Kaynağı göster
Shrink to tray|Sistem tepsisine küçült
Select a record.|Bir kayıt seçin.
Show all resources|Tüm kaynaklar
My assignments|Kendi atamalarım
Known defaults|Bilinen varsayılanlar
Detected on this computer|Bu bilgisayarda algılananlar
Computer + defaults|Bilgisayar + varsayılanlar
Check conflicts|Çakışmaları denetle
By application|Uygulamaya göre
Appearance and behavior|Görünüm ve davranış
Customize appearance|Görünümü özelleştir
Playback speed|Çalıştırma hızı
Configure AI model|Yapay zekâ modelini ayarla
Keyboard behavior|Klavye davranışı
Spotify and network|Spotify ve ağ
Data and commands|Veriler ve komutlar
Open data folder|Veri klasörünü aç
Open user guide|Kullanım kılavuzunu aç
Transfer, backup and device inspection|Aktarım, yedekleme ve cihaz incelemesi
Choose, design or import your own palette.|Paletinizi seçin, tasarlayın veya içe aktarın.
Choose a palette|Bir palet seçin
Create a theme|Tema oluştur
Import a theme|Tema içe aktar
Export selected|Seçileni dışa aktar
Edit selected|Seçileni düzenle
Delete selected|Seçileni sil
Dark|Koyu
Light|Açık
Clear history|Geçmişi temizle
Create a shortcut. Choose your input. Keep your flow.|Kısayol oluşturun, girdinizi seçin ve işinize devam edin.
Search, edit and manage your assignments.|Atamalarınızı arayın, düzenleyin ve yönetin.
Manage keyboard triggers, key combinations and keyboard-activated macros.|Klavye tetikleyicilerini, kombinasyonları ve makroları yönetin.
Assign buttons and wheel directions to your actions.|Fare tuşlarını ve tekerlek yönlerini işlemlere atayın.
Draw a shape to run an action. No activation key needed.|İşlemi çalıştırmak için bir şekil çizin. Etkinleştirme tuşu gerekmez.
Your model, your choice. Connect once and reuse it.|Modeli siz seçin. Bir kez bağlayın, tekrar kullanın.
Register an app, then give it a shortcut.|Bir uygulama ekleyin ve ona kısayol atayın.
Review overlapping triggers and application priorities.|Çakışan girdileri ve uygulama önceliklerini inceleyin.
Appearance, playback speed and input behavior.|Görünüm, çalıştırma hızı ve girdi davranışı.
Manage your workspace.|Çalışma alanınızı yönetin.
Configure model|Modeli ayarla
Choose the intelligence behind your prompts|Promptlarınızı işleyecek modeli seçin
Saved shortcuts run directly|Kayıtlı kısayollar doğrudan çalışır
Create a prompt shortcut|Prompt ile kısayol oluştur
Unload local model|Yerel modeli bellekten çıkar
LOCAL MODEL|YEREL MODEL
API SERVER|API SUNUCUSU
Prompts are sent to your configured server.|Promptlar ayarladığınız sunucuya gönderilir.
Bring a GGUF file or use a compatible API server. Makro+ does not bundle a language model.|Bir GGUF dosyası veya uyumlu API sunucusu kullanın. Makro+ dil modeli içermez.
The model is used to create a plan. Pressing a saved shortcut executes that plan immediately, without another model request.|Model bir plan oluşturmak için kullanılır. Kayıtlı kısayola basıldığında plan yeni bir model isteği olmadan hemen çalışır.
Your chosen model runs on this computer. The first request loads it; later requests reuse the session.|Seçtiğiniz model bu bilgisayarda çalışır. İlk istekte yüklenir; sonraki istekler aynı oturumu kullanır.");
  }
  static void Add(string rows){foreach(string row in rows.Replace("\r","").Split('\n')){int at=row.IndexOf('|');if(at>0)Turkish[row.Substring(0,at).Replace("\\n","\n")]=row.Substring(at+1).Replace("\\n","\n").Replace("\\s"," ");}}
  public static string T(string text){if(Language!="tr"||text==null)return text;string translated;if(Turkish.TryGetValue(text,out translated))return translated;foreach(string prefix in DynamicPrefixes)if(text.StartsWith(prefix,StringComparison.Ordinal))return Turkish[prefix]+text.Substring(prefix.Length);return text;}
  public static void Set(DependencyObject element,DependencyProperty property,string source){var map=origins.GetOrCreateValue(element);string translated=T(source);map[property]=new Origin{Source=source,Last=translated};element.SetValue(property,translated);}
  static void Translate(DependencyObject element,DependencyProperty property){if(BindingOperations.IsDataBound(element,property))return;string current=element.GetValue(property) as string;if(current==null)return;var map=origins.GetOrCreateValue(element);Origin origin;if(!map.TryGetValue(property,out origin)||current!=origin.Last)origin=new Origin{Source=current};origin.Last=T(origin.Source);map[property]=origin;if(current!=origin.Last)element.SetCurrentValue(property,origin.Last);}
  public static void Apply(DependencyObject root){
   if(root is TextBox)return; // User-entered text and code are never translated.
   if(root is TextBlock)Translate(root,TextBlock.TextProperty);
   if(root is ContentControl)Translate(root,ContentControl.ContentProperty);
   if(root is HeaderedContentControl)Translate(root,HeaderedContentControl.HeaderProperty);
   if(root is HeaderedItemsControl)Translate(root,HeaderedItemsControl.HeaderProperty);
   if(root is Window)Translate(root,Window.TitleProperty);
   if(root is FrameworkElement)Translate(root,FrameworkElement.ToolTipProperty);
   var combo=root as ComboBox;if(combo!=null&&string.IsNullOrEmpty(combo.DisplayMemberPath)&&(combo.ItemTemplate==null||combo.GetValue(TemplateLanguage)!=null)&&(string)combo.GetValue(TemplateLanguage)!=Language){var template=new DataTemplate();var text=new FrameworkElementFactory(typeof(TextBlock));text.SetBinding(TextBlock.TextProperty,new System.Windows.Data.Binding{Converter=new LocalizedLabel()});template.VisualTree=text;combo.ItemTemplate=template;combo.SetValue(TemplateLanguage,Language);}
   var grid=root as DataGrid;if(grid!=null)foreach(var column in grid.Columns)Translate(column,DataGridColumn.HeaderProperty);
   int children=VisualTreeHelper.GetChildrenCount(root);for(int i=0;i<children;i++)Apply(VisualTreeHelper.GetChild(root,i));
  }
  static bool installed;
  public static void Install(){if(installed)return;installed=true;EventManager.RegisterClassHandler(typeof(Window),FrameworkElement.LoadedEvent,new RoutedEventHandler((s,e)=>{if(ReferenceEquals(s,e.Source))Apply((Window)s);}));}
 }
}
