using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;

namespace AkilliKisayol {
 internal static class ReleaseUpdateTests {
  static readonly List<string> results=new List<string>();
  static void Check(bool value,string name){if(!value)throw new Exception(name);results.Add("PASS "+name);}
  static void Reject(Action action,string name){bool rejected=false;try{action();}catch{rejected=true;}Check(rejected,name);}
  static ReleaseVersion Version(string text){ReleaseVersion parsed;if(!ReleaseVersion.TryParse(text,out parsed))throw new Exception(text);return parsed;}
  static string Json(string tag="v3.0.0",string url=null,bool draft=false,bool pre=false){return LocalLlm.Json().Serialize(new{tag_name=tag,html_url=url??ReleaseUpdateClient.Repository+"/releases/tag/"+tag,draft=draft,prerelease=pre});}
  sealed class Handler:HttpMessageHandler {
   public Func<HttpRequestMessage,CancellationToken,Task<HttpResponseMessage>> Send;
   protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request,CancellationToken token){return Send(request,token);}
  }
  sealed class UnknownLength:HttpContent {
   readonly byte[] data;public UnknownLength(string text){data=Encoding.UTF8.GetBytes(text);}
   protected override bool TryComputeLength(out long length){length=0;return false;}
   protected override Task SerializeToStreamAsync(Stream stream,TransportContext context){return stream.WriteAsync(data,0,data.Length);}
  }
  static void Unit(string folder){
   foreach(var text in new[]{"v2.8.0","2.10.0","3.0.0.1","v3.0.0-rc.2","3.0.0+build.1"}){ReleaseVersion parsed;Check(ReleaseVersion.TryParse(text,out parsed),"Parse version "+text);}
   foreach(var text in new[]{"latest","v2.8","2.08.0","2.8.-1","https://evil.test","2.8.0/../../x","2147483648.0.0",""}){ReleaseVersion parsed;Check(!ReleaseVersion.TryParse(text,out parsed),"Reject invalid version "+text);}
   Check(Version("2.10.0").CompareTo(Version("2.9.9"))>0,"Numeric minor ordering");
   Check(Version("3.0.0").CompareTo(Version("3.0.0-rc.9"))>0,"Stable outranks prerelease");
   Check(Version("3.0.0-rc.10").CompareTo(Version("3.0.0-rc.2"))>0,"Numeric prerelease ordering");
   Check(Version("3.0.0+one").CompareTo(Version("3.0.0+two"))==0,"Ignore build metadata");
   Check(ReleaseUpdateClient.Parse(Json(),"2.8.0").Version=="3.0.0","Detect newer stable release");
   Check(ReleaseUpdateClient.Parse(Json("v2.8.0"),"2.8.0")==null,"Do not notify installed version");
   Check(ReleaseUpdateClient.Parse(Json("v2.7.0"),"2.8.0")==null,"Do not offer a downgrade");
   Check(ReleaseUpdateClient.Parse(Json(draft:true),"2.8.0")==null,"Exclude drafts");
   Check(ReleaseUpdateClient.Parse(Json(pre:true),"2.8.0")==null,"Exclude GitHub prereleases");
   Check(ReleaseUpdateClient.Parse(Json("v3.0.0-rc.1"),"2.8.0")==null,"Exclude semantic prereleases");
   Check(ReleaseUpdateClient.Parse(Json("v3.0.0+build.1"),"2.8.0")!=null,"Allow valid encoded tag metadata");
   foreach(string url in new[]{"https://evil.test/releases/tag/v3.0.0","http://github.com/yafermys/MakroPlus/releases/tag/v3.0.0","https://github.com/other/repo/releases/tag/v3.0.0",ReleaseUpdateClient.Repository+"/releases/tag/v3.0.0?download=evil",ReleaseUpdateClient.Repository+"/releases/tag/v3.0.0#x","https://user@github.com/yafermys/MakroPlus/releases/tag/v3.0.0","https://github.com:444/yafermys/MakroPlus/releases/tag/v3.0.0"})Reject(()=>ReleaseUpdateClient.Parse(Json(url:url),"2.8.0"),"Reject unexpected release URL "+url);
   foreach(string json in new[]{"{}","null","[]","{broken","{\"tag_name\":\"v3.0.0\",\"draft\":\"false\",\"prerelease\":false}"})Reject(()=>ReleaseUpdateClient.Parse(json,"2.8.0"),"Reject malformed release metadata "+json);
   var now=DateTimeOffset.UtcNow;var options=new UpdatePreferences();Check(ReleaseUpdateClient.Due(options,now),"First launch checks for releases");options.Enabled=false;Check(!ReleaseUpdateClient.Due(options,now),"Disabled automatic checks remain offline");options.Enabled=true;options.LastCheckedUtc=now.ToString("o");options.LastCheckSucceeded=true;Check(!ReleaseUpdateClient.Due(options,now.AddHours(5)),"Successful checks use six-hour cache");Check(ReleaseUpdateClient.Due(options,now.AddHours(6)),"Successful cache expires");options.LastCheckSucceeded=false;Check(!ReleaseUpdateClient.Due(options,now.AddMinutes(59)),"Failed requests back off");Check(ReleaseUpdateClient.Due(options,now.AddHours(1)),"Retry after offline backoff");options.LastCheckedUtc=now.AddDays(1).ToString("o");Check(ReleaseUpdateClient.Due(options,now),"Recover from clock moving backwards");options.LastCheckedUtc="bad";Check(ReleaseUpdateClient.Due(options,now),"Recover from invalid check timestamp");
   var release=ReleaseUpdateClient.Cached("v3.0.0","2.8.0");Check(ReleaseUpdateClient.Notify(options,release),"Notify once for a new version");Check(!ReleaseUpdateClient.Notify(options,release),"Suppress repeated notifications");Check(!ReleaseUpdateClient.Notify(options,ReleaseUpdateClient.Cached("v2.9.0","2.8.0")),"Suppress older notification after newer version");Check(ReleaseUpdateClient.Notify(options,ReleaseUpdateClient.Cached("v3.1.0","2.8.0")),"Notify a subsequent new version");Check(ReleaseUpdateClient.Cached("v2.8.0","2.8.0")==null,"Clear cached notice after installation");
   string path=Path.Combine(folder,"state.json");var state=new State{Updates=options};Storage.Write(state,path);Check(Storage.Read(path).Updates.LastNotifiedVersion=="v3.1.0","Notification suppression survives restart");Transfer.Export(state,Path.Combine(folder,"export.json"));var exported=Storage.Read(Path.Combine(folder,"export.json")).Updates;Check(exported.Enabled&&exported.LastCheckedUtc==null&&exported.LastNotifiedVersion==null&&exported.AvailableTag==null,"Export preference without machine notification cache");state.Updates=null;Storage.Write(state,path);Check(Storage.Read(path).Updates.Enabled,"Older settings receive enabled defaults");
   var handler=new Handler{Send=(request,token)=>{Check(request.Method==HttpMethod.Get&&request.RequestUri.AbsoluteUri==ReleaseUpdateClient.Endpoint,"Anonymous GET uses fixed public release endpoint");Check(request.Headers.Authorization==null&&request.Content==null,"Request contains no credentials, prompts or settings");Check(request.Headers.UserAgent.ToString().Contains(Brand.Version),"Request identifies installed app version");return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK){Content=new StringContent(Json())});}};
   using(var client=new ReleaseUpdateClient(handler))Check(client.CheckAsync("2.8.0",CancellationToken.None).GetAwaiter().GetResult().Tag=="v3.0.0","HTTP response produces release notice");
   foreach(var code in new[]{HttpStatusCode.NotFound,HttpStatusCode.Forbidden,(HttpStatusCode)429,HttpStatusCode.InternalServerError,HttpStatusCode.Redirect}){var status=code;using(var client=new ReleaseUpdateClient(new Handler{Send=(r,t)=>Task.FromResult(new HttpResponseMessage(status))}))Reject(()=>client.CheckAsync("2.8.0",CancellationToken.None).GetAwaiter().GetResult(),"Handle HTTP "+(int)code);}
   foreach(bool knownLength in new[]{true,false}){using(var client=new ReleaseUpdateClient(new Handler{Send=(r,t)=>Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK){Content=knownLength?(HttpContent)new StringContent(new string('x',131073)):new UnknownLength(new string('x',131073))})}))Reject(()=>client.CheckAsync("2.8.0",CancellationToken.None).GetAwaiter().GetResult(),"Bound release response bytes, known length="+knownLength);}
   using(var cancellation=new CancellationTokenSource())using(var client=new ReleaseUpdateClient(new Handler{Send=async(r,t)=>{await Task.Delay(30000,t);return new HttpResponseMessage(HttpStatusCode.OK);}})){cancellation.CancelAfter(20);bool cancelled=false;try{client.CheckAsync("2.8.0",cancellation.Token).GetAwaiter().GetResult();}catch(OperationCanceledException){cancelled=true;}Check(cancelled,"Cancel an in-flight network request");}
  }
  static void Pump(Task task){var limit=DateTime.UtcNow.AddSeconds(10);while(!task.IsCompleted&&DateTime.UtcNow<limit){var frame=new DispatcherFrame();Dispatcher.CurrentDispatcher.BeginInvoke(DispatcherPriority.Background,new Action(()=>frame.Continue=false));Dispatcher.PushFrame(frame);Thread.Sleep(1);}if(!task.IsCompleted)throw new TimeoutException("UI update check");task.GetAwaiter().GetResult();}
  static IEnumerable<T> Descendants<T>(DependencyObject parent) where T:DependencyObject {for(int i=0;i<VisualTreeHelper.GetChildrenCount(parent);i++){var child=VisualTreeHelper.GetChild(parent,i);var typed=child as T;if(typed!=null)yield return typed;foreach(var descendant in Descendants<T>(child))yield return descendant;}}
  static void UiCheck(string folder){
   var app=new Application{ShutdownMode=ShutdownMode.OnExplicitShutdown};Theme.Install(app);var state=new State{Paused=true,Gestures=new GestureOptions{Enabled=false},Language="en"};var window=new MainWindow(state,Path.Combine(folder,"ui.json"));app.MainWindow=window;
   try{int notifications=0;window.NotifyReleaseForTest=(a,b)=>notifications++;window.FetchReleaseForTest=t=>Task.FromResult(ReleaseUpdateClient.Cached("v3.0.0",Brand.Version));window.Show();window.ProductViewForTest("Settings");Pump(window.CheckUpdatesForTest());Check(notifications==1&&window.UpdateBannerForTest,"New release shows banner and one notification");Pump(window.CheckUpdatesForTest());Check(notifications==1,"Repeat check does not duplicate notification");window.ChangeLanguage("tr");window.ProductViewForTest("Settings");Check(Ui.T("App updates")!="App updates"&&Descendants<TextBlock>(window).Any(t=>t.Text.Contains("3.0.0")),"Turkish update settings and version banner render");
    var toggle=Descendants<CheckBox>(window).Single(c=>Equals(c.Content,Ui.T("Notify me when a new Makro+ version is available")));toggle.IsChecked=false;toggle.RaiseEvent(new RoutedEventArgs(CheckBox.ClickEvent));Check(!state.Updates.Enabled&&!Storage.Read(Path.Combine(folder,"ui.json")).Updates.Enabled,"Opt-out persists without changing shortcuts");
    window.FetchReleaseForTest=t=>Task.FromResult<AvailableRelease>(null);Pump(window.CheckUpdatesForTest());Check(!window.UpdateBannerForTest&&state.Updates.LastCheckSucceeded,"Manual checking works with automatic notifications disabled");
    window.FetchReleaseForTest=t=>{throw new HttpRequestException("fixture offline");};Pump(window.CheckUpdatesForTest());Check(!state.Updates.LastCheckSucceeded&&window.UpdateStatusForTest.Length>0,"Offline error is reported without crashing");
    window.FetchReleaseForTest=t=>Task.FromResult(ReleaseUpdateClient.Cached("v3.1.0",Brand.Version));Pump(window.CheckUpdatesForTest());Check(window.UpdateBannerForTest&&notifications==2,"Recover after offline failure and notify next version");
    window.ChangeLanguage("en");window.ProductViewForTest("Settings");window.UpdateLayout();var image=new RenderTargetBitmap((int)window.ActualWidth,(int)window.ActualHeight,96,96,PixelFormats.Pbgra32);image.Render(window);var png=new PngBitmapEncoder();png.Frames.Add(BitmapFrame.Create(image));using(var file=File.Create(Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"update-settings-preview.png")))png.Save(file);
    var entered=new TaskCompletionSource<bool>();window.FetchReleaseForTest=async token=>{entered.TrySetResult(true);await Task.Delay(30000,token);return null;};var pending=window.CheckUpdatesForTest();Pump(entered.Task);window.Close();Pump(pending);Check(pending.IsCompleted,"Closing cancels pending request without blocking shutdown");
   }finally{window.Close();app.Shutdown();}
  }
  public static int Run(bool ui=false){string folder=Path.Combine(Path.GetTempPath(),"MakroPlus-Updates-"+Guid.NewGuid().ToString("N"));Directory.CreateDirectory(folder);string report=Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"update-notifier-tests.txt");try{Unit(folder);if(ui)UiCheck(folder);File.WriteAllLines(report,new[]{"PASS: "+results.Count+" update notification checks"}.Concat(results));return 0;}catch(Exception error){File.WriteAllText(report,"FAIL after "+results.Count+" checks\n"+error);return 1;}finally{string full=Path.GetFullPath(folder),parent=Path.GetFullPath(Path.GetTempPath()).TrimEnd(Path.DirectorySeparatorChar)+Path.DirectorySeparatorChar;if(!full.StartsWith(parent,StringComparison.OrdinalIgnoreCase)||!Path.GetFileName(full).StartsWith("MakroPlus-Updates-",StringComparison.Ordinal))throw new InvalidOperationException("Unexpected test folder");Directory.Delete(full,true);}}
 }
}
