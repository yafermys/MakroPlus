# Embed the application icon and manifest into a built exe (the GNU toolchain has no windres).
param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Icon, [Parameter(Mandatory)][string]$Manifest)
$ErrorActionPreference = 'Stop'
if (-not ('ResEdit' -as [type])) {
Add-Type @'
using System; using System.IO; using System.Runtime.InteropServices;
public static class ResEdit {
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr BeginUpdateResource(string file, bool deleteExisting);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool UpdateResource(IntPtr h, IntPtr type, IntPtr name, ushort lang, byte[] data, uint size);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool EndUpdateResource(IntPtr h, bool discard);
  public static void Apply(string exe, string ico, string manifest) {
    byte[] icon = File.ReadAllBytes(ico);
    int count = BitConverter.ToUInt16(icon, 4);
    IntPtr h = BeginUpdateResource(exe, false);
    if (h == IntPtr.Zero) throw new Exception("BeginUpdateResource " + Marshal.GetLastWin32Error());
    var group = new MemoryStream(); var w = new BinaryWriter(group);
    w.Write((ushort)0); w.Write((ushort)1); w.Write((ushort)count);
    for (int i = 0; i < count; i++) {
      int o = 6 + i * 16;
      int size = BitConverter.ToInt32(icon, o + 8), offset = BitConverter.ToInt32(icon, o + 12);
      byte[] img = new byte[size]; Array.Copy(icon, offset, img, 0, size);
      if (!UpdateResource(h, (IntPtr)3, (IntPtr)(i + 1), 0, img, (uint)img.Length)) throw new Exception("RT_ICON " + Marshal.GetLastWin32Error());
      w.Write(icon, o, 12); w.Write((ushort)(i + 1));
    }
    byte[] g = group.ToArray();
    if (!UpdateResource(h, (IntPtr)14, (IntPtr)1, 0, g, (uint)g.Length)) throw new Exception("RT_GROUP_ICON " + Marshal.GetLastWin32Error());
    byte[] m = File.ReadAllBytes(manifest);
    if (!UpdateResource(h, (IntPtr)24, (IntPtr)1, 0, m, (uint)m.Length)) throw new Exception("RT_MANIFEST " + Marshal.GetLastWin32Error());
    if (!EndUpdateResource(h, false)) throw new Exception("EndUpdateResource " + Marshal.GetLastWin32Error());
  }
}
'@
}
[ResEdit]::Apply((Resolve-Path $Exe).Path, (Resolve-Path $Icon).Path, (Resolve-Path $Manifest).Path)
"Resources embedded: $Exe"
