Makro+ 3 for Linux (x86_64)

QUICK START
  ./makroplus            run it directly (portable)
  Settings > Install     adds Makro+ to your applications menu

PERMISSIONS (once)
  Makro+ reads the keyboard/mouse through /dev/input and sends keys through
  /dev/uinput. This works on X11 and Wayland but needs your user in the
  'input' group plus a udev rule:

    sudo usermod -aG input "$USER"
    sudo cp 60-makroplus.rules /etc/udev/rules.d/
    sudo udevadm control --reload && sudo udevadm trigger

  Then log out and back in. The .deb/.rpm packages install the rule for you.

WHAT WORKS ON LINUX
  Keyboard, mouse-button and wheel shortcuts; two-key shortcuts; touchpad
  shapes; workflows (keys, text, apps, files, URLs, clipboard, shell, AI);
  clipboard page; themes and backgrounds; update checks.

LIMITS
  - Keys are swallowed and can be switched off only in Exclusive mode
    (Settings > Linux permissions).
  - App-specific shortcuts need X11, Sway or Hyprland.
  - Scanning/clicking controls inside apps is Windows-only for now.
  - Typing text uses wtype (Wayland) or xdotool (X11) when installed,
    otherwise the clipboard.
  - Video backgrounds need ffmpeg.

Türkçe: Kurulum ve izinler için yukarıdaki komutları bir kez çalıştırın,
sonra oturumu kapatıp açın.
