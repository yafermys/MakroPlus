using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;

namespace AkilliKisayol {
 public sealed class SystemActionSpec {
  public string Id,Title,Description,Category; public bool Confirm;
  public SystemActionSpec(string id,string title,string description,string category,bool confirm=false){Id=id;Title=title;Description=description;Category=category;Confirm=confirm;}
  public override string ToString(){return Ui.T(Title);}
 }
 public sealed class CloseTarget {public IntPtr Handle;public int ProcessId;public string Name,Title;}
 internal interface ISystemHost {
  List<CloseTarget> Windows(); bool Close(CloseTarget target); void Start(string file,string arguments,bool shell);
  void Lock(); void Keys(int mods,int key); string CheckUpdates();
 }
 public static class SystemActions {
  public static readonly SystemActionSpec[] Catalog={
   new SystemActionSpec("lock","Lock computer","Lock your session immediately.","System"),
   new SystemActionSpec("desktop","Show desktop","Hide or restore your open windows.","Everyday"),
   new SystemActionSpec("task-view","Switch workspace","See open windows and virtual desktops.","Everyday"),
   new SystemActionSpec("screenshot","Capture a screen region","Choose an area and copy a screenshot.","Everyday"),
   new SystemActionSpec("clipboard","Clipboard history","Choose from copied items. Enable clipboard history in Windows first.","Everyday"),
   new SystemActionSpec("emoji","Emoji and symbols","Insert symbols, emoji and special characters.","Everyday"),
   new SystemActionSpec("explorer","Open File Explorer","Jump straight to your files.","Everyday"),
   new SystemActionSpec("task-manager","Open Task Manager","Inspect running applications and resource usage.","System"),
   new SystemActionSpec("audio","Sound output settings","Change speakers, headphones and input devices.","System"),
   new SystemActionSpec("network","Network settings","Manage Wi-Fi, Ethernet and connection settings.","System"),
   new SystemActionSpec("storage","Storage settings","Review disk usage and Windows cleanup options.","System"),
   new SystemActionSpec("updates","Check Windows updates","Start an online scan now. Results appear here and in history; scanning can take several minutes. Nothing is installed.","System"),
   new SystemActionSpec("update-settings","Open Windows Update","View update status and available Windows update controls.","System"),
   new SystemActionSpec("close-apps","Close open applications","Request closure of visible application windows in your session. Save dialogs are respected; background services and Makro+ stay open.","Power",true),
   new SystemActionSpec("shutdown","Shut down computer","Ask Windows to shut down after confirmation. Applications may block shutdown to save your work.","Power",true),
   new SystemActionSpec("restart","Restart computer","Ask Windows to restart after confirmation. No forced termination.","Power",true)
  };
  public static SystemActionSpec Find(string id){return Catalog.FirstOrDefault(a=>a.Id==id);}
  public static string Validate(string id){return Find(id)==null?"Choose a valid system action.":null;}
  static int updateBusy,closeBusy,powerBusy;
  internal static async Task<string> Run(string id,ISystemHost host,Func<string,bool> confirm,Action<string> progress){
   var spec=Find(id);if(spec==null)throw new ArgumentException("Unknown system action.");
   if(id=="updates"){
    if(Interlocked.CompareExchange(ref updateBusy,1,0)!=0)return Ui.T("An update scan is already running.");
    try{progress(Ui.T("Checking Windows updates… You can keep using Makro+."));return await Task.Run(()=>host.CheckUpdates());}finally{Interlocked.Exchange(ref updateBusy,0);}
   }
   if(id=="close-apps"){
    if(Interlocked.CompareExchange(ref closeBusy,1,0)!=0)return Ui.T("A close request is already in progress.");
    try{var windows=await Task.Run(()=>host.Windows());if(windows.Count==0)return Ui.T("No eligible application windows are open.");
     string question=Ui.T("Close these application windows?")+"\n\n"+string.Join("\n",windows.Take(12).Select(w=>w.Name+" — "+w.Title))+(windows.Count>12?"\n… +"+(windows.Count-12):"")+"\n\n"+Ui.T(spec.Description);
     if(!confirm(question))return Ui.T("Cancelled. No action was sent.");
     return await Task.Run(()=>{int sent=0;foreach(var window in windows)if(host.Close(window))sent++;return Ui.T("Close requests sent:")+" "+sent+" / "+windows.Count+". "+Ui.T("Save prompts may keep applications open.");});
    }finally{Interlocked.Exchange(ref closeBusy,0);}
   }
   if(spec.Confirm){
    if(Interlocked.CompareExchange(ref powerBusy,1,0)!=0)return Ui.T("A power request is already in progress.");
    try{if(!confirm(Ui.T(spec.Title)+"?\n\n"+Ui.T(spec.Description)))return Ui.T("Cancelled. No action was sent.");await Task.Run(()=>Execute(id,host));return Ui.T("Request sent to Windows.");}finally{Interlocked.Exchange(ref powerBusy,0);}
   }
   // Key injection and locking stay on the calling STA; slow OS operations use a worker.
   if(id=="lock"){host.Lock();return Ui.T("Request sent to Windows.");}
   int[] chord=Chord(id);if(chord!=null){host.Keys(chord[0],chord[1]);return Ui.T("Request sent to Windows.");}
   await Task.Run(()=>Execute(id,host));return Ui.T("Request sent to Windows.");
  }
  internal static int[] Chord(string id){switch(id){case "desktop":return new[]{8,68};case "task-view":return new[]{8,9};case "screenshot":return new[]{12,83};case "clipboard":return new[]{8,86};case "emoji":return new[]{8,190};default:return null;}}
  internal static void Execute(string id,ISystemHost host){
   string system=Environment.GetFolderPath(Environment.SpecialFolder.System);
   switch(id){
    case "shutdown":host.Start(Path.Combine(system,"shutdown.exe"),"/s /t 0",false);return;
    case "restart":host.Start(Path.Combine(system,"shutdown.exe"),"/r /t 0",false);return;
    case "task-manager":host.Start(Path.Combine(system,"Taskmgr.exe"),"",false);return;
    case "explorer":host.Start(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows),"explorer.exe"),"",false);return;
    case "audio":host.Start("ms-settings:sound","",true);return;
    case "network":host.Start("ms-settings:network","",true);return;
    case "storage":host.Start("ms-settings:storagesense","",true);return;
    case "update-settings":host.Start("ms-settings:windowsupdate","",true);return;
    default:throw new ArgumentException("Unknown system action.");
   }
  }
  public static Task<string> Run(string id,Window owner,Action<string> progress){return Run(id,new WindowsSystemHost(),text=>MessageBox.Show(owner,text,Ui.T("Confirm system action"),MessageBoxButton.YesNo,MessageBoxImage.Warning,MessageBoxResult.No)==MessageBoxResult.Yes,progress);}
 }
 internal sealed class WindowsSystemHost:ISystemHost {
  delegate bool EnumProc(IntPtr window,IntPtr param);
  [DllImport("user32.dll")]static extern bool EnumWindows(EnumProc callback,IntPtr param);
  [DllImport("user32.dll")]static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")]static extern IntPtr GetWindow(IntPtr window,uint command);
  [DllImport("user32.dll",SetLastError=true)]static extern bool PostMessage(IntPtr window,uint message,IntPtr w,IntPtr l);
  [DllImport("user32.dll",SetLastError=true)]static extern bool LockWorkStation();
  static readonly HashSet<string> excluded=new HashSet<string>(new[]{"ShellExperienceHost","StartMenuExperienceHost","SearchHost","SearchApp","TextInputHost","LockApp","dwm","sihost","CredentialUIBroker","LogonUI","winlogon","csrss","lsass","services","smss","wininit"},StringComparer.OrdinalIgnoreCase);
  static readonly HashSet<string> shellClasses=new HashSet<string>(new[]{"Progman","WorkerW","Shell_TrayWnd","Shell_SecondaryTrayWnd","#32770"},StringComparer.OrdinalIgnoreCase);
  internal static bool Eligible(string name,int pid,int session,int ownPid,int ownSession,bool visible,bool owned,string title,string windowClass=""){return visible&&!owned&&pid!=ownPid&&session==ownSession&&!string.IsNullOrWhiteSpace(title)&&!excluded.Contains(name)&&!shellClasses.Contains(windowClass);}
  public List<CloseTarget> Windows(){var result=new List<CloseTarget>();using(var own=Process.GetCurrentProcess()){int ownId=own.Id,session=own.SessionId;EnumWindows((window,param)=>{uint pid;Native.GetWindowThreadProcessId(window,out pid);try{using(var p=Process.GetProcessById((int)pid)){string title=Native.WindowTitle(window);if(Eligible(p.ProcessName,p.Id,p.SessionId,ownId,session,IsWindowVisible(window),GetWindow(window,4)!=IntPtr.Zero,title,Native.WindowClass(window)))result.Add(new CloseTarget{Handle=window,ProcessId=p.Id,Name=p.ProcessName,Title=title});}}catch(ArgumentException){}catch(InvalidOperationException){}catch(Win32Exception){}return true;},IntPtr.Zero);}return result;}
  public bool Close(CloseTarget target){uint pid;Native.GetWindowThreadProcessId(target.Handle,out pid);return pid==target.ProcessId&&PostMessage(target.Handle,0x0010,IntPtr.Zero,IntPtr.Zero);}
  public void Start(string file,string arguments,bool shell){using(var process=Process.Start(new ProcessStartInfo(file,arguments){UseShellExecute=shell,CreateNoWindow=!shell})) {}}
  public void Lock(){if(!LockWorkStation())throw new Win32Exception(Marshal.GetLastWin32Error());}
  public void Keys(int mods,int key){Native.Send(mods,key);}
  static object Call(object target,string member,params object[] arguments){return target.GetType().InvokeMember(member,BindingFlags.InvokeMethod,null,target,arguments);}
  static object Property(object target,string member){return target.GetType().InvokeMember(member,BindingFlags.GetProperty,null,target,null);}
  static void Release(object value){if(value!=null&&Marshal.IsComObject(value))Marshal.FinalReleaseComObject(value);}
  public string CheckUpdates(){object session=null,searcher=null,result=null,updates=null;try{
    var type=Type.GetTypeFromProgID("Microsoft.Update.Session",true);session=Activator.CreateInstance(type);searcher=Call(session,"CreateUpdateSearcher");
    searcher.GetType().InvokeMember("Online",BindingFlags.SetProperty,null,searcher,new object[]{true});
    result=Call(searcher,"Search","IsInstalled=0 and IsHidden=0 and Type='Software'");updates=Property(result,"Updates");int code=Convert.ToInt32(Property(result,"ResultCode")),count=Convert.ToInt32(Property(updates,"Count"));
    if(code!=2&&code!=3)throw new InvalidOperationException(Ui.T("Windows could not complete the update scan.")+" ("+code+")");
    return (code==3?Ui.T("Update scan partially completed:"):Ui.T("Available software updates:"))+" "+count+". "+Ui.T("Open Windows Update to review details. Nothing was installed.");
   }catch(TargetInvocationException error){throw new InvalidOperationException(Ui.T("Windows Update scan failed. Check your connection and Windows Update policy.")+" "+(error.InnerException??error).Message,error);}finally{Release(updates);Release(result);Release(searcher);Release(session);}}
 }
}
