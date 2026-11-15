using System;
using System.Diagnostics;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Forms=System.Windows.Forms;
namespace AkilliKisayol {
 public partial class MainWindow {
  readonly ContentControl updateNotice=new ContentControl();readonly TextBlock updateStatus=Theme.Text("",12,"Muted");
  readonly DispatcherTimer updateTimer=new DispatcherTimer();ReleaseUpdateClient updateClient;CancellationTokenSource updateRequest;bool checkingUpdates,manualUpdateCheck,updateLoop;AvailableRelease availableRelease;Button checkUpdateButton,viewReleaseButton;string updateMessage="",updateArgument;
  internal Func<CancellationToken,Task<AvailableRelease>> FetchReleaseForTest;
  internal Action<string,string> NotifyReleaseForTest;
  void InitializeUpdateChecks(bool automatic){
   if(state.Updates==null)state.Updates=new UpdatePreferences();availableRelease=ReleaseUpdateClient.Cached(state.Updates.AvailableTag,Brand.Version);RefreshUpdateNotice();
   updateLoop=automatic;updateTimer.Interval=TimeSpan.FromSeconds(15);updateTimer.Tick+=async(s,e)=>{updateTimer.Stop();try{if(ReleaseUpdateClient.Due(state.Updates,DateTimeOffset.UtcNow))await CheckRelease(false);}finally{ScheduleUpdateCheck();}};
   tray.BalloonTipClicked+=(s,e)=>{if(availableRelease!=null)OpenReleasePage();};
   Loaded+=(s,e)=>{if(updateLoop&&state.Updates.Enabled)updateTimer.Start();};
  }
  void ScheduleUpdateCheck(){updateTimer.Stop();if(!closed&&updateLoop&&state.Updates.Enabled){updateTimer.Interval=TimeSpan.FromMinutes(30);updateTimer.Start();}}
  void StopUpdateChecks(){updateTimer.Stop();if(updateRequest!=null)updateRequest.Cancel();if(updateClient!=null)updateClient.Dispose();}
  void SaveUpdateState(){historyDirty=true;if(!historySave.IsEnabled)historySave.Start();}
  async Task CheckRelease(bool manual){
   if(closed||checkingUpdates)return;if(!manual&&!state.Updates.Enabled)return;
   DateTimeOffset last;if(manual&&DateTimeOffset.TryParse(state.Updates.LastCheckedUtc,out last)&&DateTimeOffset.UtcNow>=last&&DateTimeOffset.UtcNow-last<TimeSpan.FromSeconds(30)){UpdateMessage("Please wait 30 seconds between update checks.");return;}
   checkingUpdates=true;manualUpdateCheck=manual;updateRequest=new CancellationTokenSource();if(checkUpdateButton!=null)checkUpdateButton.IsEnabled=false;UpdateMessage("Checking for a new Makro+ version…");
   try{
    if(updateClient==null&&FetchReleaseForTest==null)updateClient=new ReleaseUpdateClient();var token=updateRequest.Token;var release=await Task.Run(()=>FetchReleaseForTest!=null?FetchReleaseForTest(token):updateClient.CheckAsync(Brand.Version,token));if(closed||token.IsCancellationRequested||!manual&&!state.Updates.Enabled)return;
    state.Updates.LastCheckedUtc=DateTimeOffset.UtcNow.ToString("o");state.Updates.LastCheckSucceeded=true;state.Updates.AvailableTag=release==null?null:release.Tag;availableRelease=release;
    if(release==null)UpdateMessage("You are using the latest published version.");else{
     UpdateMessage("Makro+ {0} is available.",release.Version);if(ReleaseUpdateClient.Notify(state.Updates,release))try{string title=Ui.T("A new Makro+ version is available"),message=string.Format(Ui.T("Version {0} is ready. Click to view the release and download it."),release.Version);if(NotifyReleaseForTest!=null)NotifyReleaseForTest(title,message);else tray.ShowBalloonTip(8000,title,message,Forms.ToolTipIcon.Info);}catch(InvalidOperationException){}
    }
    SaveUpdateState();RefreshUpdateNotice();
   }catch(OperationCanceledException){if(!closed){if(updateRequest!=null&&updateRequest.IsCancellationRequested)UpdateMessage("Update check cancelled.");else UpdateCheckFailed();}}
   catch(Exception){if(!closed)UpdateCheckFailed();}
   finally{if(updateRequest!=null){updateRequest.Dispose();updateRequest=null;}checkingUpdates=false;if(!closed&&checkUpdateButton!=null)checkUpdateButton.IsEnabled=true;}
  }
  void UpdateCheckFailed(){state.Updates.LastCheckedUtc=DateTimeOffset.UtcNow.ToString("o");state.Updates.LastCheckSucceeded=false;SaveUpdateState();UpdateMessage("Could not check GitHub releases. Check your connection or try again later.");}
  void RefreshUpdateNotice(){
   updateNotice.Content=null;updateNotice.Visibility=availableRelease==null?Visibility.Collapsed:Visibility.Visible;
   if(viewReleaseButton!=null)viewReleaseButton.IsEnabled=availableRelease!=null;
   if(availableRelease==null)return;var bar=new DockPanel();var button=Theme.Button("View release",OpenReleasePage,true);DockPanel.SetDock(button,Dock.Right);bar.Children.Add(button);var text=Theme.Text(string.Format(Ui.T("Makro+ {0} is available."),availableRelease.Version),14);text.VerticalAlignment=VerticalAlignment.Center;bar.Children.Add(text);var card=Theme.Card(bar,"Selection");card.Margin=new Thickness(0,0,0,16);updateNotice.Content=card;
  }
  void OpenReleasePage(){if(availableRelease==null)return;try{Process.Start(new ProcessStartInfo(availableRelease.Url){UseShellExecute=true});}catch(Exception){UpdateMessage("The browser could not be opened. Visit the Makro+ GitHub releases page.");}}
  void UpdateMessage(string message,string argument=null){updateMessage=message;updateArgument=argument;RenderUpdateMessage();}
  void RenderUpdateMessage(){if(updateArgument==null)Ui.Set(updateStatus,TextBlock.TextProperty,updateMessage);else updateStatus.Text=string.Format(Ui.T(updateMessage),updateArgument);}
  UIElement UpdateSettings(){RenderUpdateMessage();
   var panel=new StackPanel();panel.Children.Add(Theme.Text("App updates",20));var enabled=new CheckBox{Content=Ui.T("Notify me when a new Makro+ version is available"),IsChecked=state.Updates.Enabled};panel.Children.Add(enabled);
   enabled.Click+=(s,e)=>{bool previous=state.Updates.Enabled;state.Updates.Enabled=enabled.IsChecked==true;if(!Save()){state.Updates.Enabled=previous;enabled.IsChecked=previous;return;}if(!state.Updates.Enabled){updateTimer.Stop();if(checkingUpdates&&!manualUpdateCheck&&updateRequest!=null)updateRequest.Cancel();}else{updateTimer.Interval=TimeSpan.FromSeconds(1);if(updateLoop)updateTimer.Start();}};
   panel.Children.Add(Theme.Text("Checks public GitHub releases after launch and about every six hours while running. No shortcuts or prompts are sent. Downloads and installation are your choice.",12,"Muted"));
   var buttons=new WrapPanel{Margin=new Thickness(0,10,0,10)};checkUpdateButton=Theme.Button("Check for updates now",async()=>await CheckRelease(true));checkUpdateButton.IsEnabled=!checkingUpdates;viewReleaseButton=Theme.Button("View release",OpenReleasePage);viewReleaseButton.IsEnabled=availableRelease!=null;buttons.Children.Add(checkUpdateButton);buttons.Children.Add(viewReleaseButton);panel.Children.Add(buttons);Detach(updateStatus);panel.Children.Add(updateStatus);if(state.Updates.LastCheckedUtc!=null){DateTimeOffset when;if(DateTimeOffset.TryParse(state.Updates.LastCheckedUtc,out when))panel.Children.Add(Theme.Text(string.Format(Ui.T("Last checked: {0}"),when.ToLocalTime().ToString("g")),11,"Muted"));}return panel;
  }
  internal Task CheckUpdatesForTest(){state.Updates.LastCheckedUtc=null;return CheckRelease(true);}
  internal bool UpdateBannerForTest{get{return updateNotice.Visibility==Visibility.Visible;}}
  internal string UpdateStatusForTest{get{return updateStatus.Text;}}
 }
}
