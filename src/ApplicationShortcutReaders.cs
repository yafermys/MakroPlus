using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Web.Script.Serialization;
using System.Xml;

namespace AkilliKisayol {
 public sealed class AppShortcutSource {
  public string Path,Kind,App,Name,Profile="";
  public AppShortcutSource(string path,string kind,string app,string name){Path=path;Kind=kind;App=app;Name=name;}
 }
 public static class ApplicationShortcutReaders {
  public const int MaxBytes=16*1024*1024,MaxRecords=5000;
  public static string Supported="VS Code / Insiders / Cursor / VSCodium / Antigravity, Notepad++, Sublime Text, JetBrains keymaps, VLC, OBS Studio, PowerToys Keyboard Manager, Brave shortcuts, Chromium extension commands, Minecraft, League of Legends, Counter-Strike 2";
  static Dictionary<string,object> Obj(object value){return value as Dictionary<string,object>??new Dictionary<string,object>();}
  static object Get(object value,string name){object result;return Obj(value).TryGetValue(name,out result)?result:null;}
  static string Str(object value,string name){return Get(value,name) as string??"";}
  static bool Flag(object value,string name){return Get(value,name) is bool&&(bool)Get(value,name);}
  static object[] Array(object value){return value as object[]??new object[0];}
  static string ReadText(string path){using(var stream=new FileStream(path,FileMode.Open,FileAccess.Read,FileShare.ReadWrite|FileShare.Delete)){if(stream.Length>MaxBytes)throw new InvalidDataException("Shortcut source exceeds the 16 MB scan limit.");using(var reader=new StreamReader(stream,System.Text.Encoding.UTF8,true)){var buffer=new char[4096];var text=new System.Text.StringBuilder();int read;while((read=reader.Read(buffer,0,buffer.Length))>0){if(text.Length+read>MaxBytes)throw new InvalidDataException("Shortcut source grew beyond the scan limit.");text.Append(buffer,0,read);}return text.ToString();}}}
  static SystemShortcut Item(AppShortcutSource source,int ordinal,string name,string key,string command,string context="",bool enabled=true,string scope=null){
   string normalized=Normalize(key);int mods,vk;bool single=ShortcutKeys.TryCode(normalized,out mods,out vk);return new SystemShortcut{Id="app:"+source.Kind+":"+System.IO.Path.GetFullPath(source.Path)+"#"+ordinal,Provider="App: "+source.Kind,ApplicationName=source.Name,Name=name,App=scope??source.App,Profile=source.Profile,FilePath=source.Path,KeyText=normalized,Key=single?vk:0,Mods=single?mods:0,Command=command,When=context,Enabled=enabled&&normalized.Length>0};
  }
  internal static string Normalize(string key){
   key=key.Trim().Replace("Control+","Ctrl+").Replace("control+","ctrl+").Replace("Super+","Win+").Replace("Command+","Win+");
   var aliases=new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase){{"Return","Enter"},{"Escape","Esc"},{"Prior","PageUp"},{"Next","PageDown"},{"Left Button","LButton"},{"Right Button","RButton"},{"Middle Button","MButton"},{"Mouse Button 4","XButton1"},{"Mouse Button 5","XButton2"},{"Spacebar","Space"},{"MouseWheelUp","WheelUp"},{"MouseWheelDown","WheelDown"}};
   string[] strokes=Regex.Split(key,@"\s+");for(int i=0;i<strokes.Length;i++){string[] parts=strokes[i].Split('+');for(int j=0;j<parts.Length;j++){string alias;if(aliases.TryGetValue(parts[j],out alias))parts[j]=alias;}string chord=string.Join("+",parts);int m,k;strokes[i]=ShortcutKeys.TryCode(chord,out m,out k)?ShortcutKeys.Code(m,k):chord;}return string.Join(" ",strokes);
  }
  public static List<SystemShortcut> Read(AppShortcutSource source){
   string text=ReadText(source.Path);var result=new List<SystemShortcut>();Action<string,string,string,string,bool,string> add=(name,key,command,when,enabled,scope)=>{if(result.Count>=MaxRecords)throw new InvalidDataException("Shortcut source exceeds 5000 assignments.");result.Add(Item(source,result.Count,name,key,command,when,enabled,scope));};
   if(source.Kind=="VLC"){
    foreach(string line in text.Split('\n')){string row=line.Trim();if(row.Length==0||row[0]=='#'||row[0]==';')continue;int eq=row.IndexOf('=');if(eq<0)continue;string command=row.Substring(0,eq).Trim();if(!command.StartsWith("key-")&&!command.StartsWith("global-key-"))continue;foreach(string key in row.Substring(eq+1).Split('\t'))add(command,key.Trim(),command,"",key.Trim().Length>0,command.StartsWith("global-")?"":source.App);}return result;
   }
   if(source.Kind=="Minecraft"){
    foreach(string row in text.Split('\n')){if(!row.StartsWith("key_",StringComparison.Ordinal))continue;int colon=row.IndexOf(':');if(colon<0)continue;string command=row.Substring(0,colon),raw=row.Substring(colon+1).Trim();string key=raw.Replace("key.keyboard.","").Replace("key.mouse.","Mouse:");var names=new Dictionary<string,string>{{"Mouse:left","LButton"},{"Mouse:right","RButton"},{"Mouse:middle","MButton"},{"Mouse:4","XButton1"},{"Mouse:5","XButton2"},{"left.shift","LShift"},{"right.shift","RShift"},{"left.control","LCtrl"},{"right.control","RCtrl"},{"left.alt","LAlt"},{"right.alt","RAlt"},{"grave.accent","`"},{"left.bracket","["},{"right.bracket","]"},{"equal","="},{"apostrophe","'"},{"key.keyboard.unknown",""},{"unknown",""}};string known;if(names.TryGetValue(key,out known))key=known;else key=key.Replace("page.up","PageUp").Replace("page.down","PageDown").Replace("caps.lock","CapsLock").Replace("num.lock","NumLock");add(command.Replace("key_key.","").Replace("key_",""),key,command,"Game input; physical/modifier keys may not map to Windows chords",key.Length>0,null);}return result;
   }
   if(source.Kind=="League of Legends"){
    string section="";foreach(string line in text.Split('\n')){string row=line.Trim();if(row.StartsWith("[")){section=row;continue;}if(!row.StartsWith("evt",StringComparison.Ordinal))continue;int eq=row.IndexOf('=');if(eq<0)continue;string command=row.Substring(0,eq);var keys=Regex.Matches(row.Substring(eq+1),@"\[([^\]]*)\]");if(keys.Count==0&&row.Substring(eq+1).Trim().Length==0)add(command,"",command,section,false,null);foreach(Match match in keys){string key=LeagueKey(match.Groups[1].Value);add(command,key,command,section,key.Length>0,null);}}return result;
   }
   if(source.Kind=="Counter-Strike 2"){
    bool bindings=false;int depth=0;foreach(string line in text.Split('\n')){string row=line.Trim();if(row.StartsWith("//"))continue;if(row=="\"bindings\""){bindings=true;continue;}if(!bindings)continue;if(row=="{"){depth++;continue;}if(row=="}"){if(--depth<=0)bindings=false;continue;}var match=Regex.Match(row,"^\"([^\"]+)\"\\s+\"((?:\\\\.|[^\"\\\\])*)\"$");if(!match.Success||depth!=1)continue;string raw=match.Groups[1].Value;string key=raw.ToUpperInvariant()=="MOUSE1"?"LButton":raw.ToUpperInvariant()=="MOUSE2"?"RButton":raw.ToUpperInvariant()=="MOUSE3"?"MButton":raw.ToUpperInvariant()=="MOUSE4"?"XButton1":raw.ToUpperInvariant()=="MOUSE5"?"XButton2":raw.ToUpperInvariant()=="MWHEELUP"?"WheelUp":raw.ToUpperInvariant()=="MWHEELDOWN"?"WheelDown":raw;string command=match.Groups[2].Value;add(command,key,command,"Game context; scancode bindings are shown literally",command.Length>0,null);}return result;
   }
   if(source.Kind=="Notepad++"||source.Kind=="JetBrains"){
    var doc=new XmlDocument{XmlResolver=null};var settings=new XmlReaderSettings{DtdProcessing=DtdProcessing.Prohibit,XmlResolver=null,MaxCharactersInDocument=MaxBytes};using(var reader=XmlReader.Create(new StringReader(text),settings))doc.Load(reader);
    if(source.Kind=="Notepad++"){
     if(doc.DocumentElement==null||doc.DocumentElement.Name!="NotepadPlus")throw new InvalidDataException("Expected a Notepad++ shortcuts.xml file.");
     foreach(XmlElement node in doc.SelectNodes("//*[@Key]")){int key;if(!int.TryParse(node.GetAttribute("Key"),out key)||key<0||key>255)continue;int mods=(node.GetAttribute("Ctrl")=="yes"?2:0)|(node.GetAttribute("Alt")=="yes"?1:0)|(node.GetAttribute("Shift")=="yes"?4:0);var parent=node.ParentNode as XmlElement;string id=node.GetAttribute("id");if(id.Length==0&&parent!=null)id=parent.GetAttribute("ScintID");string name=node.GetAttribute("name");if(name.Length==0)name=node.Name+" "+id;add(name,key==0?"":ShortcutKeys.Code(mods,key),node.Name+" "+id,"",key!=0,null);}
    }else{
     if(doc.DocumentElement==null||doc.DocumentElement.Name!="keymap")throw new InvalidDataException("Expected a JetBrains keymap export.");
     foreach(XmlElement node in doc.SelectNodes("//action/keyboard-shortcut")){var action=(XmlElement)node.ParentNode;Func<string,string> stroke=value=>Regex.Replace(value,@"\bpressed\s+","").Trim().Replace("control ","ctrl ").Replace(" ","+");string key=stroke(node.GetAttribute("first-keystroke")),second=stroke(node.GetAttribute("second-keystroke"));if(second.Length>0)key+=" "+second;add(action.GetAttribute("id"),key,action.GetAttribute("id"),"Custom keymap; inherited defaults are not included",key.Length>0,null);}
    }return result;
   }
   if(source.Kind=="OBS Studio"&&System.IO.Path.GetExtension(source.Path).Equals(".ini",StringComparison.OrdinalIgnoreCase)){
    bool hotkeys=false;foreach(string row in text.Split('\n')){string line=row.Trim();if(line.StartsWith("[")){hotkeys=line.Equals("[Hotkeys]",StringComparison.OrdinalIgnoreCase);continue;}if(!hotkeys)continue;int at=line.IndexOf('=');if(at<1)continue;var json=new JavaScriptSerializer{MaxJsonLength=MaxBytes,RecursionLimit=64}.DeserializeObject(line.Substring(at+1));ReadObs(json,line.Substring(0,at),add);}return result;
   }
   var serializer=new JavaScriptSerializer{MaxJsonLength=MaxBytes,RecursionLimit=64};object data=serializer.DeserializeObject(source.Kind=="Sublime Text"?SystemShortcuts.Jsonc(text):text);
   if(source.Kind=="Sublime Text"){
    if(!(data is object[]))throw new InvalidDataException("Expected a Sublime keymap array.");foreach(object rule in Array(data)){string key=string.Join(" ",Array(Get(rule,"keys")).OfType<string>());string command=Str(rule,"command");add(command.Length>0?command:"Blocked key",key,command,Get(rule,"context")!=null?"Conditional Sublime context":"",key.Length>0,null);}return result;
   }
   if(!(data is Dictionary<string,object>))throw new InvalidDataException("Expected a shortcut settings object.");
   if(source.Kind=="Brave shortcuts"){
    var current=Obj(Get(Get(data,"brave"),"accelerators"));var defaults=Obj(Get(Get(data,"brave"),"default_accelerators"));foreach(var pair in current){if(!(pair.Value is object[]))throw new InvalidDataException("Unexpected Brave accelerator format.");string name=BraveCommandName(pair.Key);var bindings=Array(pair.Value);object defaultKeys;bool custom=defaults.TryGetValue(pair.Key,out defaultKeys)&&!bindings.OfType<string>().SequenceEqual(Array(defaultKeys).OfType<string>());if(bindings.Length==0)add(name,"","Brave command "+pair.Key,"Disabled Brave shortcut",false,source.App);foreach(string raw in bindings.OfType<string>())add(name,BraveKey(raw),"Brave command "+pair.Key,custom?"Customized Brave shortcut":"Saved Brave shortcut; profile/layout may affect behavior",raw.Length>0,source.App);}
   }
   if(source.Kind=="Chromium extensions"||source.Kind=="Brave shortcuts"){
    foreach(var pair in Obj(Get(Get(data,"extensions"),"commands"))){if(!pair.Key.StartsWith("windows:",StringComparison.Ordinal))continue;string[] fields=pair.Key.Split(':');string command=Str(pair.Value,"command_name");string extension=Str(pair.Value,"extension");object state=Get(Get(Get(Get(data,"extensions"),"settings"),extension),"state");bool enabled=!(state is int)||(int)state==1;add(command.Length>0?command:"Extension command",fields.Length>1?fields[1]:"",command,"Browser profile: "+source.Profile,enabled,Flag(pair.Value,"global")?"":source.App);}return result;
   }
   if(source.Kind=="PowerToys"){
    foreach(string group in new[]{"remapKeys","remapShortcuts"})foreach(string section in new[]{"inProcess","global","appSpecific"})foreach(var rule in Array(Get(Get(data,group),section))){string input=PowerKeys(Str(rule,"originalKeys")),output=PowerKeys(Str(rule,"newRemapKeys"));string app=Str(rule,"targetApp");if(app.Length>0&&!app.EndsWith(".exe",StringComparison.OrdinalIgnoreCase))app+=".exe";add("Remap to "+output,input,output,"PowerToys profile; active/running state is not verified",input.Length>0,app);}return result;
   }
   if(source.Kind=="OBS Studio"){WalkObs(data,add,0);return result;}
   throw new InvalidDataException("Unsupported application shortcut format.");
  }
  static string LeagueKey(string text){if(text==" ")return "Space";if(text.Equals("<Unbound>",StringComparison.OrdinalIgnoreCase))return "";for(int button=1;button<=5;button++)text=text.Replace("Button "+button,new[]{"LButton","RButton","MButton","XButton1","XButton2"}[button-1]);return Regex.Replace(text,@"\b(Ctrl|Shift|Alt)\s+","$1+",RegexOptions.IgnoreCase);}
  static string BraveKey(string text){var keys=text.Split('+');var aliases=new Dictionary<string,string>{{"Control","Ctrl"},{"Meta","Win"},{"ArrowLeft","Left"},{"ArrowRight","Right"},{"ArrowUp","Up"},{"ArrowDown","Down"},{"Equal","oem_plus"},{"Minus","oem_minus"},{"BracketLeft","oem_4"},{"BracketRight","oem_6"},{"Backslash","oem_5"},{"Slash","oem_2"},{"Backquote","oem_3"},{"Quote","oem_7"},{"Comma","oem_comma"},{"Period","oem_period"},{"Semicolon","oem_1"}};for(int i=0;i<keys.Length;i++){string alias;if(aliases.TryGetValue(keys[i],out alias))keys[i]=alias;else if(Regex.IsMatch(keys[i],"^Key[A-Z]$"))keys[i]=keys[i].Substring(3);else if(Regex.IsMatch(keys[i],"^Digit[0-9]$"))keys[i]=keys[i].Substring(5);}return string.Join("+",keys);}
  // Public command identifiers; unrecognized IDs remain visible rather than being guessed.
  // https://chromium.googlesource.com/chromium/src/+/main/chrome/app/chrome_command_ids.h
  static readonly Dictionary<string,string> BraveNames=LoadBraveNames();
  static Dictionary<string,string> LoadBraveNames(){var names=new Dictionary<string,string>{{"33000","Back"},{"33001","Forward"},{"33002","Reload"},{"33003","Home"},{"33004","Open current address"},{"33006","Stop loading"},{"33007","Reload without cache"},{"33009","Reload and clear cache"},{"34000","New window"},{"34001","New private window"},{"34012","Close window"},{"34014","New tab"},{"34015","Close tab"},{"34016","Next tab"},{"34017","Previous tab"},{"34026","Last tab"},{"34027","Duplicate tab"},{"34028","Reopen closed tab"},{"34030","Fullscreen"},{"34031","Exit"},{"34032","Move tab right"},{"34033","Move tab left"},{"34035","Search"},{"34046","Minimize window"},{"34047","Maximize window"}};foreach(string line in @"34018=Tab 1
34019=Tab 2
34020=Tab 3
34021=Tab 4
34022=Tab 5
34023=Tab 6
34024=Tab 7
34025=Tab 8
34057=New split tab
34061=Collapse vertical tabs
34062=Cycle to next tab
34063=Cycle to previous tab
34100=Add tab to group
34101=Create tab group
34102=Next tab group
34103=Previous tab group
34104=Close tab group
35000=Bookmark this tab
35001=Bookmark all tabs
35002=Page source
35003=Print
35004=Save page
35007=System print dialog
35031=Screenshot
37000=Find in page
37001=Find next
37002=Find previous
37003=Close find or stop
38001=Zoom in
38002=Reset zoom
38003=Zoom out
39000=Focus toolbar
39001=Address bar
39002=Focus search
39003=Focus menu bar
39004=Next pane
39005=Previous pane
39006=Focus bookmarks
39007=Focus popup
39009=Focus web content
40000=Open file
40004=Developer tools
40005=Developer console
40009=Bookmarks bar
40010=Browsing history
40011=Bookmark manager
40012=Downloads
40013=Clear browsing data
40019=Browser help
40021=App menu
40023=Inspect element
40134=Profile menu
40237=Toggle developer tools
40260=Caret browsing
40286=Browser task manager
40303=Reading mode
52500=Tab search
56003=New private Tor window
56041=Mute or unmute tab
56044=Toggle sidebar
56301=Command menu".Split('\n')){int split=line.IndexOf('=');names[line.Substring(0,split).Trim()]=line.Substring(split+1).Trim();}
return names;}
  static string BraveCommandName(string id){string label;return BraveNames.TryGetValue(id,out label)?label:"Brave command "+id;}
  static string PowerKeys(string text){var keys=new List<int>();foreach(string part in text.Split(';')){int key;if(!int.TryParse(part,out key)||key<1||key>255)return text;keys.Add(key);}int mods=0;foreach(int key in keys)mods|=key==16||key==160||key==161?4:key==17||key==162||key==163?2:key==18||key==164||key==165?1:key==91||key==92?8:0;var main=keys.Where(k=>!Native.Modifier(k)).ToArray();return main.Length==1?ShortcutKeys.Code(mods,main[0]):text;}
  static void ReadObs(object value,string command,Action<string,string,string,string,bool,string> add){object binds=Get(value,"bindings")??value;foreach(var binding in Array(binds)){string key=Str(binding,"key").Replace("OBS_KEY_","");if(key=="NONE"||key.Length==0){add(command,"",command,"OBS unassigned hotkey",false,"");continue;}key=key=="MOUSE1"?"LButton":key=="MOUSE2"?"RButton":key=="MOUSE3"?"MButton":key=="MOUSE4"?"XButton1":key=="MOUSE5"?"XButton2":key;key=(Flag(binding,"control")?"Ctrl+":"")+(Flag(binding,"alt")?"Alt+":"")+(Flag(binding,"shift")?"Shift+":"")+(Flag(binding,"command")?"Win+":"")+key;add(command,key,command,"OBS profile/scene hotkey; active collection is not verified",key.Length>0,"");}}
  static void WalkObs(object value,Action<string,string,string,string,bool,string> add,int depth){if(depth>32)return;var obj=value as Dictionary<string,object>;if(obj!=null){foreach(var pair in obj){if(pair.Key=="hotkeys"){foreach(var command in Obj(pair.Value))ReadObs(command.Value,command.Key,add);}else if(pair.Key=="sources"||pair.Key=="groups")foreach(var child in Array(pair.Value))WalkObs(child,add,depth+1);}}}
 }
}
