using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
namespace AkilliKisayol {
 public sealed class ApplicationShortcutEntry {
  public DiscoveredApplication Application;
  public int Detected,Personal,References;
  public string Coverage="";
  public string Label{get{return Application.Name+"  ·  "+(Detected+Personal)+(References>0?" + "+References+" ref":"");}}
  public override string ToString(){return Label;}
 }
 public static class ApplicationIndex {
  static string Token(string text){return Regex.Replace((text??"").ToLowerInvariant(),@"[^\p{L}\p{Nd}+]","");}
  public static string Identity(string name,string executable){
   string key=!string.IsNullOrEmpty(executable)&&executable.EndsWith(".exe",StringComparison.OrdinalIgnoreCase)?Token(Path.GetFileNameWithoutExtension(executable)):Token(Regex.Replace(name??"",@"\s+\d+(?:\.\d+)+(?:.*)?$",""));
   if(key=="javaw"&&Token(name)=="minecraft")return "minecraft";if(key=="codeinsiders")return "codeinsiders";if(key=="googlechrome"||key=="chrome")return "chrome";if(key=="microsoftedge"||key=="msedge")return "msedge";if(key=="leagueoflegends")return "leagueoflegends";if(key.StartsWith("discord"))return "discord";if(key=="counterstrike2"||key=="cs2")return "cs2";if(key=="obsstudio"||key=="obs64")return "obs64";if(key=="sublimetext"||key=="sublimetext3"||key=="sublime_text")return "sublimetext";if(key=="visualstudiocode"||key=="code")return "code";return key;
  }
  static string Owner(SystemShortcut item){if(!string.IsNullOrEmpty(item.ApplicationName))return Identity(item.ApplicationName,item.ApplicationName=="Minecraft"||item.ApplicationName=="PowerToys"?"":item.App);if(item.Provider=="Windows .lnk")return Identity(AppLibrary.Name(item.Target),item.Target);return Identity(AppLibrary.Name(item.App),item.App);}
  public static bool Matches(DiscoveredApplication app,SystemShortcut item,bool globals=false){
   if(Owner(item)==app.Id)return true;
   if(item.App.Length>0&&Identity(AppLibrary.Name(item.App),item.App)==app.Id)return true;
   return globals&&item.App.Length==0;
  }
  public static bool Matches(DiscoveredApplication app,Binding binding,bool globals=false){
   Func<string,bool> target=path=>!string.IsNullOrWhiteSpace(path)&&Identity(AppLibrary.Name(path),path)==app.Id;
   return target(binding.AppPath)||binding.Kind=="Application"&&target(binding.Target)||binding.Kind=="Browser website"&&target(binding.BrowserPath)||(binding.Steps??new List<ActionStep>()).Any(s=>target(s.AppPath))||globals&&binding.AppPath.Length==0;
  }
  public static List<ApplicationShortcutEntry> Build(IEnumerable<DiscoveredApplication> installed,IEnumerable<Binding> bindings,IEnumerable<SystemShortcut> external,IEnumerable<AppSourceResult> sources){
   var records=external.Where(i=>!i.Reference).ToArray();var own=bindings.ToArray();var apps=new List<DiscoveredApplication>(installed);
   foreach(var item in records){string name=item.ApplicationName.Length>0?item.ApplicationName:item.Provider=="Windows .lnk"?AppLibrary.Name(item.Target):AppLibrary.Name(item.App);string exe=item.Provider=="Windows .lnk"?item.Target:item.App;if(name.Length>0)apps.Add(new DiscoveredApplication{Name=name,Executable=item.ApplicationName=="PowerToys"?"PowerToys.exe":exe});if(item.ApplicationName=="PowerToys"&&item.App.Length>0)apps.Add(new DiscoveredApplication{Name=AppLibrary.Name(item.App),Executable=item.App});}
   foreach(var b in own)foreach(string path in new[]{b.AppPath,b.Kind=="Application"?b.Target:"",b.BrowserPath??""}.Concat((b.Steps??new List<ActionStep>()).Select(s=>s.AppPath??"")))if(path.Length>0)apps.Add(new DiscoveredApplication{Name=AppLibrary.Name(path),Executable=path});
   foreach(var report in sources)apps.Add(new DiscoveredApplication{Name=report.Source.Name,Executable=report.Source.App});
   var detectedCounts=new Dictionary<string,int>(StringComparer.OrdinalIgnoreCase);var personalCounts=new Dictionary<string,int>(StringComparer.OrdinalIgnoreCase);
   foreach(var item in records.Where(i=>i.Enabled)){var ids=new HashSet<string>(StringComparer.OrdinalIgnoreCase){Owner(item)};if(item.App.Length>0)ids.Add(Identity(AppLibrary.Name(item.App),item.App));foreach(string id in ids){int count;detectedCounts.TryGetValue(id,out count);detectedCounts[id]=count+1;}}
   foreach(var binding in own){var ids=new HashSet<string>(new[]{binding.AppPath,binding.Kind=="Application"?binding.Target:"",binding.BrowserPath??""}.Concat((binding.Steps??new List<ActionStep>()).Select(s=>s.AppPath??"")).Where(p=>p.Length>0).Select(p=>Identity(AppLibrary.Name(p),p)),StringComparer.OrdinalIgnoreCase);foreach(string id in ids){int count;personalCounts.TryGetValue(id,out count);personalCounts[id]=count+1;}}
   var referenceCounts=ShortcutConflicts.References().Where(r=>r.App.Length>0).GroupBy(r=>Identity(AppLibrary.Name(r.App),r.App)).ToDictionary(g=>g.Key,g=>g.Count());
   var reportsById=sources.GroupBy(s=>Identity(s.Source.Name,s.Source.Name=="Minecraft"?"":s.Source.App)).ToDictionary(g=>g.Key,g=>g.ToArray());
   var codeSources=new HashSet<string>(records.Where(i=>i.Provider=="Application settings").Select(Owner),StringComparer.OrdinalIgnoreCase);var menuSources=new HashSet<string>(records.Where(i=>i.Provider=="Live app menu").Select(Owner),StringComparer.OrdinalIgnoreCase);
   return apps.Where(a=>a.Name.Length>0&&a.Id.Length>0).GroupBy(a=>a.Id,StringComparer.OrdinalIgnoreCase).Select(g=>{
    var app=g.OrderByDescending(a=>Path.IsPathRooted(a.Executable)).ThenByDescending(a=>a.Name.Length).First();AppSourceResult[] reports;if(!reportsById.TryGetValue(app.Id,out reports))reports=new AppSourceResult[0];
    string coverage=reports.Any(s=>!s.Success)?"Some sources could not be read":reports.Any(s=>s.Source.Kind=="Brave shortcuts")?"Saved Brave shortcuts and extension commands scanned":reports.Any(s=>s.Source.Kind=="Chromium extensions")?"Saved extension commands only; browser defaults are not scanned":reports.Any(s=>s.Success)?"Supported settings files scanned":codeSources.Contains(app.Id)?"User keybindings scanned":menuSources.Contains(app.Id)?"Open-window menu shortcuts read":"No readable app settings source found";
    int detected,personal,references;referenceCounts.TryGetValue(app.Id,out references);detectedCounts.TryGetValue(app.Id,out detected);personalCounts.TryGetValue(app.Id,out personal);return new ApplicationShortcutEntry{Application=app,Detected=detected,Personal=personal,Coverage=coverage,References=references};
   }).OrderByDescending(a=>a.Detected+a.Personal>0).ThenBy(a=>a.Application.Name,StringComparer.CurrentCultureIgnoreCase).ToList();
  }
 }
}
