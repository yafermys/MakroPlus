using System;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Runtime.Serialization;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Collections.Generic;

namespace AkilliKisayol {
 [DataContract] public sealed class UpdatePreferences {
  [DataMember] public bool Enabled=true;
  [DataMember(EmitDefaultValue=false)] public string LastCheckedUtc,LastNotifiedVersion,AvailableTag;
  [DataMember(EmitDefaultValue=false)] public bool LastCheckSucceeded;
  public UpdatePreferences Copy(){return (UpdatePreferences)MemberwiseClone();}
  public UpdatePreferences Export(){return new UpdatePreferences{Enabled=Enabled};}
 }
 public sealed class ReleaseVersion:IComparable<ReleaseVersion> {
  public int Major,Minor,Patch,Revision;public string Pre="";
  public static bool TryParse(string text,out ReleaseVersion version){version=null;if(text==null||text.Length>100)return false;var match=Regex.Match(text,@"^[vV]?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$");int major,minor,patch,revision=0;if(!match.Success||!int.TryParse(match.Groups[1].Value,out major)||!int.TryParse(match.Groups[2].Value,out minor)||!int.TryParse(match.Groups[3].Value,out patch)||match.Groups[4].Success&&!int.TryParse(match.Groups[4].Value,out revision))return false;version=new ReleaseVersion{Major=major,Minor=minor,Patch=patch,Revision=revision,Pre=match.Groups[5].Value};return true;}
  public int CompareTo(ReleaseVersion other){if(other==null)return 1;int compare=Major.CompareTo(other.Major);if(compare==0)compare=Minor.CompareTo(other.Minor);if(compare==0)compare=Patch.CompareTo(other.Patch);if(compare==0)compare=Revision.CompareTo(other.Revision);if(compare!=0)return compare;if(Pre==other.Pre)return 0;if(Pre.Length==0)return 1;if(other.Pre.Length==0)return -1;var left=Pre.Split('.');var right=other.Pre.Split('.');for(int i=0;i<Math.Min(left.Length,right.Length);i++){long a,b;bool an=long.TryParse(left[i],out a),bn=long.TryParse(right[i],out b);compare=an&&bn?a.CompareTo(b):an?-1:bn?1:string.CompareOrdinal(left[i],right[i]);if(compare!=0)return compare;}return left.Length.CompareTo(right.Length);}
 }
 public sealed class AvailableRelease {
  public string Tag,Version,Url;
 }
 public sealed class ReleaseUpdateClient:IDisposable {
  public const string Repository="https://github.com/yafermys/MakroPlus",Endpoint="https://api.github.com/repos/yafermys/MakroPlus/releases/latest";
  readonly HttpClient client;
  public ReleaseUpdateClient(HttpMessageHandler handler=null){client=handler==null?new HttpClient(new HttpClientHandler{AllowAutoRedirect=false,UseCookies=false}):new HttpClient(handler);client.Timeout=TimeSpan.FromSeconds(10);client.DefaultRequestHeaders.UserAgent.ParseAdd("MakroPlus/"+Brand.Version);client.DefaultRequestHeaders.Accept.ParseAdd("application/vnd.github+json");client.DefaultRequestHeaders.Add("X-GitHub-Api-Version","2022-11-28");}
  public static AvailableRelease Cached(string tag,string current){ReleaseVersion candidate,installed;if(!ReleaseVersion.TryParse(tag,out candidate)||candidate.Pre.Length>0||!ReleaseVersion.TryParse(current,out installed)||candidate.CompareTo(installed)<=0)return null;return new AvailableRelease{Tag=tag,Version=tag.TrimStart('v','V'),Url=Repository+"/releases/tag/"+Uri.EscapeDataString(tag)};}
  public static AvailableRelease Parse(string json,string current){var data=new JavaScriptSerializer{MaxJsonLength=131072,RecursionLimit=32}.DeserializeObject(json) as Dictionary<string,object>;if(data==null)throw new InvalidDataException("Unexpected release response.");object raw;string tag=data.TryGetValue("tag_name",out raw)?raw as string:null;ReleaseVersion version;if(!ReleaseVersion.TryParse(tag,out version))throw new InvalidDataException("Invalid release version.");foreach(string flag in new[]{"draft","prerelease"})if(!data.TryGetValue(flag,out raw)||!(raw is bool))throw new InvalidDataException("Incomplete release metadata.");if((bool)data["draft"]||(bool)data["prerelease"]||version.Pre.Length>0)return null;var release=Cached(tag,current);if(release==null)return null;string url=data.TryGetValue("html_url",out raw)?raw as string:null;Uri address;if(!Uri.TryCreate(url,UriKind.Absolute,out address)||address.Scheme!="https"||address.Host!="github.com"||address.UserInfo.Length>0||address.Query.Length>0||address.Fragment.Length>0||!address.IsDefaultPort||Uri.UnescapeDataString(address.AbsolutePath)!=new Uri(Repository).AbsolutePath+"/releases/tag/"+tag)throw new InvalidDataException("Unexpected release page.");return release;}
  public async Task<AvailableRelease> CheckAsync(string current,CancellationToken token){using(var deadline=CancellationTokenSource.CreateLinkedTokenSource(token)){deadline.CancelAfter(TimeSpan.FromSeconds(10));return await FetchAsync(current,deadline.Token).ConfigureAwait(false);}}
  async Task<AvailableRelease> FetchAsync(string current,CancellationToken token){
   using(var request=new HttpRequestMessage(HttpMethod.Get,Endpoint))using(var response=await client.SendAsync(request,HttpCompletionOption.ResponseHeadersRead,token).ConfigureAwait(false)){
    if(response.StatusCode==HttpStatusCode.NotFound)throw new InvalidOperationException("No published release is available.");
    if(response.StatusCode==HttpStatusCode.Forbidden||(int)response.StatusCode==429)throw new InvalidOperationException("GitHub request limit reached. Try again later.");response.EnsureSuccessStatusCode();
    if(response.Content.Headers.ContentLength.HasValue&&response.Content.Headers.ContentLength.Value>131072)throw new InvalidDataException("Release response is too large.");
    using(var stream=await response.Content.ReadAsStreamAsync().ConfigureAwait(false))using(var bytes=new MemoryStream())using(token.Register(()=>stream.Dispose())){var buffer=new byte[4096];int count;try{while((count=await stream.ReadAsync(buffer,0,buffer.Length,token).ConfigureAwait(false))>0){if(bytes.Length+count>131072)throw new InvalidDataException("Release response is too large.");bytes.Write(buffer,0,count);}token.ThrowIfCancellationRequested();return Parse(Encoding.UTF8.GetString(bytes.ToArray()),current);}catch{token.ThrowIfCancellationRequested();throw;}}
   }
  }
  public static bool Due(UpdatePreferences options,DateTimeOffset now){if(options==null||!options.Enabled)return false;DateTimeOffset last;if(!DateTimeOffset.TryParse(options.LastCheckedUtc,System.Globalization.CultureInfo.InvariantCulture,System.Globalization.DateTimeStyles.RoundtripKind,out last))return true;return last>now.AddMinutes(5)||now-last>=TimeSpan.FromHours(options.LastCheckSucceeded?6:1);}
  public static bool Notify(UpdatePreferences options,AvailableRelease release){if(release==null)return false;ReleaseVersion last,current;if(ReleaseVersion.TryParse(options.LastNotifiedVersion,out last)&&ReleaseVersion.TryParse(release.Tag,out current)&&last.CompareTo(current)>=0)return false;options.LastNotifiedVersion=release.Tag;return true;}
  public void Dispose(){client.Dispose();}
 }
}
