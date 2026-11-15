using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security;
using System.Text;
using Microsoft.Win32;

namespace AkilliKisayol {
 public sealed class DiscoveredApplication {
  public string Name,Executable="";
  public string Id{get{return ApplicationIndex.Identity(Name,Executable);}}
 }
 public sealed class AppSourceResult {
  public AppShortcutSource Source;public int Count;public bool Success;public string Error="";
 }
 public static class ApplicationShortcutDiscovery {
  static IEnumerable<string> Directories(string root,string pattern="*"){if(!Directory.Exists(root))return new string[0];try{return Directory.GetDirectories(root,pattern).Where(p=>(File.GetAttributes(p)&FileAttributes.ReparsePoint)==0).Take(100).ToArray();}catch(IOException){return new string[0];}catch(UnauthorizedAccessException){return new string[0];}}
  internal static List<AppShortcutSource> Sources(string roaming,string local,IEnumerable<string> installed,bool systemLocations=true){
   var result=new List<AppShortcutSource>();Action<string,string,string,string> add=(path,kind,app,name)=>{if(File.Exists(path))result.Add(new AppShortcutSource(path,kind,app,name){Profile=Path.GetDirectoryName(path)});};
   add(Path.Combine(roaming,"Notepad++","shortcuts.xml"),"Notepad++","notepad++.exe","Notepad++");
   foreach(string product in new[]{"Sublime Text","Sublime Text 3"})foreach(string path in SafeFiles(Path.Combine(roaming,product,"Packages","User"),"*.sublime-keymap"))add(path,"Sublime Text","sublime_text.exe","Sublime Text");
   foreach(string folder in Directories(Path.Combine(roaming,"JetBrains")))foreach(string file in SafeFiles(Path.Combine(folder,"keymaps"),"*.xml"))add(file,"JetBrains",JetBrainsExe(Path.GetFileName(folder)),Path.GetFileName(folder));
   add(Path.Combine(roaming,"vlc","vlcrc"),"VLC","vlc.exe","VLC");
   foreach(string profile in Directories(Path.Combine(roaming,"obs-studio","basic","profiles")))add(Path.Combine(profile,"basic.ini"),"OBS Studio","obs64.exe","OBS Studio");
   foreach(string file in SafeFiles(Path.Combine(roaming,"obs-studio","basic","scenes"),"*.json"))add(file,"OBS Studio","obs64.exe","OBS Studio");
   foreach(string file in SafeFiles(Path.Combine(local,"Microsoft","PowerToys","Keyboard Manager"),"*.json"))if(!Path.GetFileName(file).Equals("settings.json",StringComparison.OrdinalIgnoreCase))add(file,"PowerToys","PowerToys.exe","PowerToys");
   foreach(var browser in new[]{new[]{"BraveSoftware\\Brave-Browser\\User Data","brave.exe","Brave"},new[]{"Google\\Chrome\\User Data","chrome.exe","Google Chrome"},new[]{"Microsoft\\Edge\\User Data","msedge.exe","Microsoft Edge"},new[]{"Vivaldi\\User Data","vivaldi.exe","Vivaldi"}})foreach(string profile in Directories(Path.Combine(local,browser[0])).Where(p=>Path.GetFileName(p)=="Default"||Path.GetFileName(p).StartsWith("Profile ",StringComparison.Ordinal)))add(Path.Combine(profile,"Preferences"),browser[1]=="brave.exe"?"Brave shortcuts":"Chromium extensions",browser[1],browser[2]);
   add(Path.Combine(roaming,".minecraft","options.txt"),"Minecraft","javaw.exe","Minecraft");
   foreach(string path in installed??new string[0]){string dir=Directory.Exists(path)?path:Path.GetDirectoryName(path);if(string.IsNullOrEmpty(dir))continue;
    add(Path.Combine(dir,"shortcuts.xml"),"Notepad++","notepad++.exe","Notepad++");
    if(dir.IndexOf("League of Legends",StringComparison.OrdinalIgnoreCase)>=0){add(Path.Combine(dir,"Config","input.ini"),"League of Legends","League of Legends.exe","League of Legends");if(Path.GetFileName(dir).Equals("Game",StringComparison.OrdinalIgnoreCase))add(Path.Combine(Path.GetDirectoryName(dir),"Config","input.ini"),"League of Legends","League of Legends.exe","League of Legends");}
    if(Path.GetFileName(path).Equals("options.txt",StringComparison.OrdinalIgnoreCase))add(path,"Minecraft","javaw.exe","Minecraft");
   }
   if(!systemLocations)return result.GroupBy(s=>Path.GetFullPath(s.Path),StringComparer.OrdinalIgnoreCase).Select(g=>g.First()).Take(500).ToList();
   string league=Path.Combine(Path.GetPathRoot(Environment.GetFolderPath(Environment.SpecialFolder.Windows)),"Riot Games","League of Legends","Config","input.ini");add(league,"League of Legends","League of Legends.exe","League of Legends");
   var steamRoots=new HashSet<string>(StringComparer.OrdinalIgnoreCase){Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),"Steam")};try{using(var key=Registry.CurrentUser.OpenSubKey("Software\\Valve\\Steam")){string path=key==null?"":key.GetValue("SteamPath") as string;if(!string.IsNullOrWhiteSpace(path))steamRoots.Add(path);}}catch(SecurityException){}
   foreach(string root in steamRoots)foreach(string user in Directories(Path.Combine(root,"userdata")))foreach(string file in SafeFiles(Path.Combine(user,"730","local","cfg"),"cs2_user_keys*.vcfg"))add(file,"Counter-Strike 2","cs2.exe","Counter-Strike 2");
   return result.GroupBy(s=>Path.GetFullPath(s.Path),StringComparer.OrdinalIgnoreCase).Select(g=>g.First()).Take(500).ToList();
  }
  static IEnumerable<string> SafeFiles(string folder,string pattern){try{return Directory.Exists(folder)?Directory.GetFiles(folder,pattern).Take(100).ToArray():new string[0];}catch(IOException){return new string[0];}catch(UnauthorizedAccessException){return new string[0];}}
  static string JetBrainsExe(string name){string lower=name.ToLowerInvariant();return lower.StartsWith("pycharm")?"pycharm64.exe":lower.StartsWith("webstorm")?"webstorm64.exe":lower.StartsWith("rider")?"rider64.exe":lower.StartsWith("clion")?"clion64.exe":lower.StartsWith("goland")?"goland64.exe":"idea64.exe";}
  internal static AppShortcutSource Recognize(string path){string name=Path.GetFileName(path).ToLowerInvariant();string directory=path.ToLowerInvariant();if(name=="shortcuts.xml")return new AppShortcutSource(path,"Notepad++","notepad++.exe","Notepad++");if(name.EndsWith(".sublime-keymap"))return new AppShortcutSource(path,"Sublime Text","sublime_text.exe","Sublime Text");if(name=="vlcrc")return new AppShortcutSource(path,"VLC","vlc.exe","VLC");if(name=="options.txt")return new AppShortcutSource(path,"Minecraft","javaw.exe","Minecraft");if(name=="input.ini"&&directory.Contains("league"))return new AppShortcutSource(path,"League of Legends","League of Legends.exe","League of Legends");if(name.EndsWith(".vcfg"))return new AppShortcutSource(path,"Counter-Strike 2","cs2.exe","Counter-Strike 2");if(directory.Contains("keymaps")&&name.EndsWith(".xml"))return new AppShortcutSource(path,"JetBrains",JetBrainsExe(directory),"JetBrains");if(directory.Contains("obs-studio")&&(name=="basic.ini"||name.EndsWith(".json")))return new AppShortcutSource(path,"OBS Studio","obs64.exe","OBS Studio");if(name=="preferences"&&(directory.Contains("brave")||directory.Contains("chrome")||directory.Contains("edge")||directory.Contains("vivaldi")))return new AppShortcutSource(path,directory.Contains("brave")?"Brave shortcuts":"Chromium extensions",directory.Contains("brave")?"brave.exe":directory.Contains("edge")?"msedge.exe":directory.Contains("vivaldi")?"vivaldi.exe":"chrome.exe",directory.Contains("brave")?"Brave":directory.Contains("edge")?"Microsoft Edge":directory.Contains("vivaldi")?"Vivaldi":"Google Chrome");if(directory.Contains("keyboard manager")&&name.EndsWith(".json")&&name!="settings.json")return new AppShortcutSource(path,"PowerToys","PowerToys.exe","PowerToys");return null;}
  internal static void Collect(ShortcutScan scan,IEnumerable<string> extras,IEnumerable<string> registered,bool defaults=true){
   var installed=defaults?Installed(registered):new List<DiscoveredApplication>();scan.Applications.AddRange(installed);var paths=installed.Select(a=>a.Executable).Concat(registered??new string[0]).Concat(extras??new string[0]).Where(p=>!string.IsNullOrWhiteSpace(p)).ToArray();var sources=defaults?Sources(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),paths):new List<AppShortcutSource>();
   foreach(string extra in extras??new string[0]){if(File.Exists(extra)){var source=Recognize(extra);if(source!=null)sources.Add(source);}else if(Directory.Exists(extra))foreach(string file in ExtraFiles(extra)){var source=Recognize(file);if(source!=null)sources.Add(source);}}
   foreach(var source in sources.GroupBy(s=>Path.GetFullPath(s.Path),StringComparer.OrdinalIgnoreCase).Select(g=>g.First()).Take(500)){
    var report=new AppSourceResult{Source=source};scan.AppSources.Add(report);try{var items=ApplicationShortcutReaders.Read(source);if(scan.Items.Count+items.Count>20000)throw new InvalidDataException("Application scan reached 20000 records; narrow the selected sources.");scan.Items.AddRange(items);report.Count=items.Count;report.Success=true;scan.Files++;}catch(Exception error){if(!Expected(error))throw;report.Error=error.Message;scan.Warnings.Add(source.Name+" • "+source.Path+": "+error.Message);}
   }
   if(defaults)try{scan.Items.AddRange(NativeMenuShortcuts.Read().Take(Math.Max(0,20000-scan.Items.Count)));}catch(Exception error){if(!Expected(error))throw;scan.Warnings.Add("Native menus: "+error.Message);}
  }
  static IEnumerable<string> ExtraFiles(string root){var pending=new Stack<Tuple<string,int>>();pending.Push(Tuple.Create(root,0));var files=new List<string>();int folders=0;while(pending.Count>0&&++folders<=500&&files.Count<500){var item=pending.Pop();foreach(string file in SafeFiles(item.Item1,"*"))if(Recognize(file)!=null)files.Add(file);if(item.Item2<5)foreach(string child in Directories(item.Item1))pending.Push(Tuple.Create(child,item.Item2+1));}return files;}
  internal static bool Expected(Exception error){return error is InvalidDataException||error is IOException||error is UnauthorizedAccessException||error is SecurityException||error is ArgumentException||error is InvalidOperationException||error is System.Xml.XmlException||error is System.ComponentModel.Win32Exception;}
  public static List<DiscoveredApplication> Installed(IEnumerable<string> registered=null){
   var result=new List<DiscoveredApplication>();foreach(string path in registered??new string[0])if(!string.IsNullOrWhiteSpace(path))result.Add(new DiscoveredApplication{Name=AppLibrary.Name(path),Executable=path});
   foreach(var hive in new[]{RegistryHive.CurrentUser,RegistryHive.LocalMachine})foreach(var view in new[]{RegistryView.Registry32,RegistryView.Registry64})try{using(var root=RegistryKey.OpenBaseKey(hive,view))using(var uninstall=root.OpenSubKey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall")){if(uninstall==null)continue;foreach(string sub in uninstall.GetSubKeyNames().Take(2000))try{using(var key=uninstall.OpenSubKey(sub)){if(key==null)continue;string name=key.GetValue("DisplayName") as string;if(string.IsNullOrWhiteSpace(name)||name.StartsWith("Microsoft .NET")||name.StartsWith("Microsoft Visual C++")||name.StartsWith("Microsoft ASP.NET")||name.Contains("Redistributable"))continue;string path=IconPath(key.GetValue("DisplayIcon") as string??"");string folder=key.GetValue("InstallLocation") as string??"";if(path.IndexOfAny(Path.GetInvalidPathChars())>=0)path="";if(folder.IndexOfAny(Path.GetInvalidPathChars())>=0)folder="";result.Add(new DiscoveredApplication{Name=name,Executable=path.Length>0?path:folder});}}catch(SecurityException){}catch(UnauthorizedAccessException){}}}catch(SecurityException){}catch(UnauthorizedAccessException){}catch(IOException){}
   return result.GroupBy(a=>a.Id,StringComparer.OrdinalIgnoreCase).Select(g=>g.OrderByDescending(a=>a.Executable.EndsWith(".exe",StringComparison.OrdinalIgnoreCase)).First()).OrderBy(a=>a.Name,StringComparer.CurrentCultureIgnoreCase).Take(1000).ToList();
  }
  static string IconPath(string text){text=Environment.ExpandEnvironmentVariables(text.Trim());if(text.StartsWith("\"")){int end=text.IndexOf('"',1);return end>1?text.Substring(1,end-1):"";}int icon=text.LastIndexOf(',');if(icon>=0)text=text.Substring(0,icon);return text.EndsWith(".exe",StringComparison.OrdinalIgnoreCase)?text:"";}
 }
 public static class NativeMenuShortcuts {
  delegate bool EnumProc(IntPtr window,IntPtr data);
  [DllImport("user32.dll")]static extern bool EnumWindows(EnumProc callback,IntPtr data);
  [DllImport("user32.dll")]static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")]static extern IntPtr GetMenu(IntPtr window);
  [DllImport("user32.dll")]static extern IntPtr GetSubMenu(IntPtr menu,int position);
  [DllImport("user32.dll")]static extern int GetMenuItemCount(IntPtr menu);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern int GetMenuString(IntPtr menu,uint position,StringBuilder text,int length,uint flags);
  [DllImport("user32.dll")]static extern uint GetMenuState(IntPtr menu,uint position,uint flags);
  public static List<SystemShortcut> Read(){var result=new List<SystemShortcut>();int windows=0;EnumWindows((window,param)=>{if(++windows>1000||result.Count>=5000)return false;if(!IsWindowVisible(window))return true;IntPtr menu=GetMenu(window);if(menu==IntPtr.Zero)return true;string app=Native.ProcessPath(window);if(app.Length==0)return true;Walk(menu,app,result,new HashSet<IntPtr>(),0);return true;},IntPtr.Zero);return result.GroupBy(i=>i.Id,StringComparer.OrdinalIgnoreCase).Select(g=>g.First()).ToList();}
  internal static List<SystemShortcut> ReadWindow(IntPtr window){var result=new List<SystemShortcut>();IntPtr menu=GetMenu(window);string app=Native.ProcessPath(window);if(menu!=IntPtr.Zero&&app.Length>0)Walk(menu,app,result,new HashSet<IntPtr>(),0);return result;}
  static void Walk(IntPtr menu,string app,List<SystemShortcut> result,HashSet<IntPtr> visited,int depth){if(depth>12||!visited.Add(menu))return;int count=Math.Min(500,GetMenuItemCount(menu));for(int i=0;i<count&&result.Count<5000;i++){var text=new StringBuilder(1024);GetMenuString(menu,(uint)i,text,text.Capacity,0x400);string label=text.ToString();int tab=label.LastIndexOf('\t');if(tab>0){string name=label.Substring(0,tab).Replace("&&","\u0001").Replace("&","").Replace("\u0001","&"),key=ApplicationShortcutReaders.Normalize(label.Substring(tab+1));if(key.Length>0)result.Add(new SystemShortcut{Id="menu:"+app+":"+name+":"+key,Provider="Live app menu",ApplicationName=AppLibrary.Name(app),Name=name,App=app,KeyText=key,Command=name,When="Menu label read from an open window; runtime behavior not verified"+((GetMenuState(menu,(uint)i,0x400)&3)!=0?"; command currently unavailable":""),Enabled=true});}IntPtr child=GetSubMenu(menu,i);if(child!=IntPtr.Zero)Walk(child,app,result,visited,depth+1);}}
 }
}
