using System;
using System.Collections.Generic;
namespace AkilliKisayol {
 public static class ApplicationShortcutReferences {
  // Reference entries are never presented as settings discovered on this PC.
  public static IEnumerable<SystemShortcut> Items(){
   foreach(var entry in new[]{
    new[]{"Spotify.exe","Search Spotify","ctrl+k"},new[]{"Spotify.exe","Shortcut help","ctrl+/"},new[]{"Spotify.exe","Shuffle","ctrl+s"},new[]{"Spotify.exe","Repeat","ctrl+r"},new[]{"Spotify.exe","Preferences","ctrl+,"},new[]{"Spotify.exe","Library","alt+shift+0"},new[]{"Spotify.exe","Playlists","alt+shift+1"},new[]{"Spotify.exe","Queue","alt+shift+q"},
    new[]{"Discord.exe","Mute microphone","ctrl+shift+m"},new[]{"Discord.exe","Deafen audio","ctrl+shift+d"},new[]{"Discord.exe","Find a conversation","ctrl+k"},new[]{"Discord.exe","Show hotkeys","ctrl+/"},new[]{"Discord.exe","Previous channel","alt+up"},new[]{"Discord.exe","Next channel","alt+down"},new[]{"Discord.exe","Previous server","ctrl+alt+up"},new[]{"Discord.exe","Next server","ctrl+alt+down"},new[]{"Discord.exe","Upload file","ctrl+shift+u"},new[]{"Discord.exe","Answer call","ctrl+enter"}
   }){int mods,key;if(!ShortcutKeys.TryCode(entry[2],out mods,out key))continue;yield return new SystemShortcut{Id="app-default:"+entry[0]+":"+entry[2],Provider="Known default",ApplicationName=System.IO.Path.GetFileNameWithoutExtension(entry[0]),Name=entry[1],App=entry[0],KeyText=entry[2],Key=key,Mods=mods,Command=entry[1],When="Documented default; customized/current app settings were not read",Reference=true};}
  }
 }
}
