use serde::Serialize;
use std::{collections::HashMap, fs, path::{Path, PathBuf}, sync::{Arc, RwLock}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tokio::{sync::broadcast, time::interval};

const TICK_MS: u64 = 1000;
#[derive(Serialize)] struct Snapshot { version: &'static str, timestamp: u64, hostname: String, cpu: Cpu, memory: Memory, filesystems: Vec<Filesystem>, block: Vec<Block>, network: Vec<Net>, sensors: Vec<Sensor>, gpu: Vec<Gpu>, batteries: Vec<Battery>, pressure: Pressure, system: System, top_processes: Vec<Process> }
#[derive(Clone, Serialize)] struct Process { pid: u32, name: String, cpu_percent: Option<f64>, rss_mib: f64 }
#[derive(Serialize)] struct Cpu { model: String, usage_percent: Option<f64>, per_core_percent: Vec<Option<f64>>, clock_mhz: Vec<Option<u64>>, states_percent: HashMap<String,f64>, context_switches_s: Option<f64>, interrupts_s: Option<f64> }
#[derive(Serialize)] struct Memory { total_bytes: Option<u64>, available_bytes: Option<u64>, used_bytes: Option<u64>, cached_bytes: Option<u64>, buffers_bytes: Option<u64>, active_bytes: Option<u64>, inactive_bytes: Option<u64>, slab_bytes: Option<u64>, dirty_bytes: Option<u64>, swap_total_bytes: Option<u64>, swap_used_bytes: Option<u64>, swap_in_pages_s: Option<f64>, swap_out_pages_s: Option<f64>, page_faults_s: Option<f64>, major_faults_s: Option<f64>, pgscan_s: Option<f64>, pgsteal_s: Option<f64>, compact_stall_s: Option<f64> }
#[derive(Serialize)] struct Filesystem { mount: String, source: String, kind: String, total_bytes: u64, used_bytes: u64, available_bytes: u64 }
#[derive(Serialize)] struct Block { name: String, read_bytes_s: Option<f64>, write_bytes_s: Option<f64>, read_iops: Option<f64>, write_iops: Option<f64>, busy_percent: Option<f64>, average_queue_depth: Option<f64>, read_latency_ms: Option<f64>, write_latency_ms: Option<f64> }
#[derive(Serialize)] struct Net { name: String, rx_bytes_s: Option<f64>, tx_bytes_s: Option<f64>, rx_packets_s: Option<f64>, tx_packets_s: Option<f64>, rx_errors: u64, tx_errors: u64, rx_drops: u64, tx_drops: u64, link_up: Option<bool>, speed_mbps: Option<u64> }
#[derive(Serialize)] struct Sensor { chip: String, label: String, kind: String, value: f64, unit: String }
#[derive(Serialize)] struct Gpu { name: String, driver: Option<String>, vendor_id: Option<String>, device_id: Option<String>, usage_percent: Option<f64>, vram_total_bytes: Option<u64>, vram_used_bytes: Option<u64>, core_clock_mhz: Option<u64> }
#[derive(Serialize)] struct Battery { name: String, status: Option<String>, capacity_percent: Option<u64>, power_w: Option<f64>, energy_wh: Option<f64>, ac_online: Option<bool> }
#[derive(Serialize)] struct Pressure { cpu: Option<f64>, memory: Option<f64>, io: Option<f64> }
#[derive(Serialize)] struct System { kernel: String, distro: String, uptime_s: Option<f64>, load: Vec<f64>, processes: Option<u64>, running: Option<u64>, tcp_retransmits_s: Option<f64>, tcp_established: Option<u64>, forks_s: Option<f64>, blocked_processes: Option<u64> }
#[derive(Clone)] struct Counters { cpu: Vec<Vec<u64>>, ctxt: u64, intr: u64, forks: u64, blocked: u64, net: HashMap<String,Vec<u64>>, disk: HashMap<String,Vec<u64>>, vm: HashMap<String,u64>, retrans: Option<u64> }
fn read(p: impl AsRef<Path>) -> Option<String> { fs::read_to_string(p).ok().map(|v| v.trim().to_string()) }
fn num(p: impl AsRef<Path>) -> Option<u64> { read(p)?.parse().ok() }
fn fields(s: &str) -> Vec<u64> { s.split_whitespace().filter_map(|x| x.parse().ok()).collect() }
fn map_colon(p: &str) -> HashMap<String,u64> { read(p).unwrap_or_default().lines().filter_map(|l| { let (k,v)=l.split_once(':')?; Some((k.trim().into(),v.split_whitespace().next()?.parse().ok()?)) }).collect() }
fn kb(m: &HashMap<String,u64>, k: &str) -> Option<u64> { m.get(k).and_then(|v|v.checked_mul(1024)) }
fn delta(a: Option<u64>, b: Option<u64>, sec: f64) -> Option<f64> { Some(b?.checked_sub(a?)? as f64/sec) }
fn at(v:&[u64],i:usize)->Option<u64>{v.get(i).copied()}
fn pct(a:Option<u64>,b:Option<u64>)->Option<f64>{let b=b?; if b==0 {None} else {Some(a? as f64*100.0/b as f64)}}
fn counter_snapshot() -> Counters {
 let stat=read("/proc/stat").unwrap_or_default(); let mut cpu=Vec::new();let mut ctxt=0;let mut intr=0;let mut forks=0;let mut blocked=0;
 for l in stat.lines(){let mut it=l.split_whitespace();match it.next(){Some(k) if k=="cpu" || k.strip_prefix("cpu").is_some_and(|n| !n.is_empty() && n.chars().all(|c|c.is_ascii_digit()))=>cpu.push(fields(&it.collect::<Vec<_>>().join(" "))),Some("ctxt")=>ctxt=it.next().and_then(|v|v.parse().ok()).unwrap_or(0),Some("intr")=>intr=it.next().and_then(|v|v.parse().ok()).unwrap_or(0),Some("processes")=>forks=it.next().and_then(|v|v.parse().ok()).unwrap_or(0),Some("procs_blocked")=>blocked=it.next().and_then(|v|v.parse().ok()).unwrap_or(0),_=>()}}
 let net=read("/proc/net/dev").unwrap_or_default().lines().skip(2).filter_map(|l| {let (n,v)=l.split_once(':')?;Some((n.trim().to_string(),fields(v)))}).collect();
 let disk=read("/proc/diskstats").unwrap_or_default().lines().filter_map(|l|{let mut it=l.split_whitespace();it.next()?;it.next()?;let n=it.next()?.to_string();let v=fields(&it.collect::<Vec<_>>().join(" "));if v.len()<11 {None}else{Some((n,v))}}).collect();
 let vm=read("/proc/vmstat").unwrap_or_default().lines().filter_map(|l|{let mut it=l.split_whitespace();Some((it.next()?.to_string(),it.next()?.parse().ok()?))}).collect();
 let retrans=read("/proc/net/snmp").and_then(|s| {let mut lines=s.lines().filter(|l|l.starts_with("Tcp:"));let keys=lines.next()?;let vals=lines.next()?;let pos=keys.split_whitespace().position(|v|v=="RetransSegs")?;vals.split_whitespace().nth(pos)?.parse().ok()});
 Counters{cpu,ctxt,intr,forks,blocked,net,disk,vm,retrans}
}
fn cpu_usage(a:&[u64],b:&[u64])->Option<f64>{let total_a:u64=a.iter().sum();let total_b:u64=b.iter().sum();let dt=total_b.checked_sub(total_a)?;if dt==0{return None}let idle_a=at(a,3)?.saturating_add(at(a,4).unwrap_or(0));let idle_b=at(b,3)?.saturating_add(at(b,4).unwrap_or(0));Some((100.0*(1.0-idle_b.saturating_sub(idle_a) as f64/dt as f64)).clamp(0.0,100.0))}
fn cpu_model()->String{read("/proc/cpuinfo").and_then(|s|s.lines().find(|l|l.starts_with("model name")||l.starts_with("Hardware")).and_then(|l|l.split_once(':')).map(|(_,v)|v.trim().to_string())).unwrap_or_else(||"CPU".into())}
fn cpu_clocks(n:usize)->Vec<Option<u64>>{(0..n).map(|i|num(format!("/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq")).map(|v|v/1000)).collect()}
fn memory(now:&Counters,prev:&Counters,sec:f64)->Memory{let m=map_colon("/proc/meminfo");let t=kb(&m,"MemTotal");let avail=kb(&m,"MemAvailable");let st=kb(&m,"SwapTotal");let sf=kb(&m,"SwapFree");Memory{total_bytes:t,available_bytes:avail,used_bytes:t.zip(avail).map(|(a,b)|a.saturating_sub(b)),cached_bytes:kb(&m,"Cached"),buffers_bytes:kb(&m,"Buffers"),active_bytes:kb(&m,"Active"),inactive_bytes:kb(&m,"Inactive"),slab_bytes:kb(&m,"Slab"),dirty_bytes:kb(&m,"Dirty"),swap_total_bytes:st,swap_used_bytes:st.zip(sf).map(|(a,b)|a.saturating_sub(b)),swap_in_pages_s:delta(prev.vm.get("pswpin").copied(),now.vm.get("pswpin").copied(),sec),swap_out_pages_s:delta(prev.vm.get("pswpout").copied(),now.vm.get("pswpout").copied(),sec),page_faults_s:delta(prev.vm.get("pgfault").copied(),now.vm.get("pgfault").copied(),sec),major_faults_s:delta(prev.vm.get("pgmajfault").copied(),now.vm.get("pgmajfault").copied(),sec),pgscan_s:delta(prev.vm.get("pgscan_direct").copied(),now.vm.get("pgscan_direct").copied(),sec),pgsteal_s:delta(prev.vm.get("pgsteal_direct").copied(),now.vm.get("pgsteal_direct").copied(),sec),compact_stall_s:delta(prev.vm.get("compact_stall").copied(),now.vm.get("compact_stall").copied(),sec)}}
fn filesystem(path:&str)->Option<(u64,u64,u64)>{use std::{ffi::CString,mem::MaybeUninit};let p=CString::new(path).ok()?;let mut s=MaybeUninit::<libc::statvfs>::uninit();if unsafe{libc::statvfs(p.as_ptr(),s.as_mut_ptr())}!=0{return None}let s=unsafe{s.assume_init()};let total=s.f_blocks.saturating_mul(s.f_frsize);let free=s.f_bfree.saturating_mul(s.f_frsize);let avail=s.f_bavail.saturating_mul(s.f_frsize);Some((total,total.saturating_sub(free),avail))}
fn mounted()->Vec<Filesystem>{read("/proc/self/mounts").unwrap_or_default().lines().filter_map(|l|{let p:Vec<_>=l.split_whitespace().collect();if p.len()<3||!["ext4","btrfs","xfs","vfat","exfat","ntfs","f2fs","zfs"].contains(&p[2]){return None}let mount=p[1].replace("\\040"," ");let (total,used,available)=filesystem(&mount)?;Some(Filesystem{mount,source:p[0].into(),kind:p[2].into(),total_bytes:total,used_bytes:used,available_bytes:available})}).collect()}
fn blocks(now:&Counters,prev:&Counters,sec:f64)->Vec<Block>{let mut out=Vec::new();for(n,v)in &now.disk{if !Path::new(&format!("/sys/block/{n}")).exists(){continue}let Some(p)=prev.disk.get(n) else{continue};let d=|i|delta(at(p,i),at(v,i),sec);let rs=d(0);let ws=d(4);out.push(Block{name:n.clone(),read_bytes_s:d(2).map(|x|x*512.0),write_bytes_s:d(6).map(|x|x*512.0),read_iops:rs,write_iops:ws,busy_percent:d(9).map(|x|(x/10.0).clamp(0.0,100.0)),average_queue_depth:d(10).map(|x|x/1000.0),read_latency_ms:rs.filter(|x|*x>0.0).and_then(|x|d(3).map(|ms|ms/x)),write_latency_ms:ws.filter(|x|*x>0.0).and_then(|x|d(7).map(|ms|ms/x))})}out.sort_by(|a,b|a.name.cmp(&b.name));out}
fn networks(now:&Counters,prev:&Counters,sec:f64)->Vec<Net>{let mut out=Vec::new();for(n,v)in &now.net{if n=="lo"{continue}let p=prev.net.get(n);let d=|i|delta(p.and_then(|p|at(p,i)),at(v,i),sec);let root=format!("/sys/class/net/{n}");out.push(Net{name:n.clone(),rx_bytes_s:d(0),tx_bytes_s:d(8),rx_packets_s:d(1),tx_packets_s:d(9),rx_errors:at(v,2).unwrap_or(0),tx_errors:at(v,10).unwrap_or(0),rx_drops:at(v,3).unwrap_or(0),tx_drops:at(v,11).unwrap_or(0),link_up:read(format!("{root}/operstate")).map(|v|v=="up"),speed_mbps:num(format!("{root}/speed"))})}out.sort_by(|a,b|a.name.cmp(&b.name));out}
#[derive(Clone)]struct SensorPath{chip:String,label:String,kind:String,unit:String,path:PathBuf,scale:f64}
fn discover_sensors()->Vec<SensorPath>{let mut out=Vec::new();if let Ok(dirs)=fs::read_dir("/sys/class/hwmon"){for d in dirs.flatten(){let root=d.path();let chip=read(root.join("name")).unwrap_or_default();if let Ok(files)=fs::read_dir(&root){for f in files.flatten(){let name=f.file_name().to_string_lossy().into_owned();for (prefix,unit,scale) in [("temp","°C",1000.0),("fan","RPM",1.0),("in","V",1000.0),("power","W",1_000_000.0),("curr","A",1000.0)]{let Some(idx)=name.strip_prefix(prefix).and_then(|v|v.strip_suffix("_input")) else{continue};if !idx.chars().all(|c|c.is_ascii_digit()) {continue}let label=read(root.join(format!("{prefix}{idx}_label"))).unwrap_or_else(||format!("{prefix}{idx}"));out.push(SensorPath{chip:chip.clone(),label,kind:prefix.into(),unit:unit.into(),path:f.path(),scale})}}}}}out.sort_by(|a,b|a.chip.cmp(&b.chip).then(a.label.cmp(&b.label)));out}
fn sensors(paths:&[SensorPath])->Vec<Sensor>{paths.iter().filter_map(|p|{let v=read(&p.path)?.parse::<f64>().ok()?;Some(Sensor{chip:p.chip.clone(),label:p.label.clone(),kind:p.kind.clone(),value:v/p.scale,unit:p.unit.clone()})}).collect()}
fn gpus()->Vec<Gpu>{let mut out=Vec::new();if let Ok(entries)=fs::read_dir("/sys/class/drm"){for e in entries.flatten(){let name=e.file_name().to_string_lossy().into_owned();if !name.starts_with("card")||!name[4..].chars().all(|c|c.is_ascii_digit()){continue}let dev=e.path().join("device");if !dev.exists(){continue}let driver=fs::read_link(dev.join("driver")).ok().and_then(|p|p.file_name().map(|s|s.to_string_lossy().into_owned()));out.push(Gpu{name,driver,vendor_id:read(dev.join("vendor")),device_id:read(dev.join("device")),usage_percent:num(dev.join("gpu_busy_percent")).map(|v|v as f64),vram_total_bytes:num(dev.join("mem_info_vram_total")),vram_used_bytes:num(dev.join("mem_info_vram_used")),core_clock_mhz:None})}}out}
fn batteries()->Vec<Battery>{let mut ac=None;let mut paths=Vec::new();if let Ok(entries)=fs::read_dir("/sys/class/power_supply"){for e in entries.flatten(){let p=e.path();match read(p.join("type")).as_deref(){Some("Mains")=>ac=num(p.join("online")).map(|v|v!=0),Some("Battery")=>paths.push(p),_=>()}}}paths.into_iter().map(|p|{let voltage=num(p.join("voltage_now"));let power=num(p.join("power_now")).map(|v|v as f64/1_000_000.0).or_else(||num(p.join("current_now")).zip(voltage).map(|(i,v)|i as f64*v as f64/1e12));Battery{name:p.file_name().unwrap_or_default().to_string_lossy().into_owned(),status:read(p.join("status")),capacity_percent:num(p.join("capacity")),power_w:power,energy_wh:num(p.join("energy_now")).map(|v|v as f64/1e6).or_else(||num(p.join("charge_now")).zip(voltage).map(|(c,v)|c as f64*v as f64/1e12)),ac_online:ac}}).collect()}
fn psi(name:&str)->Option<f64>{read(format!("/proc/pressure/{name}"))?.lines().find(|l|l.starts_with("some "))?.split_whitespace().find_map(|v|v.strip_prefix("avg10=").and_then(|v|v.parse().ok()))}
fn system(now:&Counters,prev:&Counters,sec:f64)->System{let load=read("/proc/loadavg").unwrap_or_default();let parts:Vec<_>=load.split_whitespace().collect();let distro=read("/etc/os-release").unwrap_or_default().lines().find_map(|l|l.strip_prefix("PRETTY_NAME=").map(|v|v.trim_matches('"').to_string())).unwrap_or_default();let snmp=read("/proc/net/snmp").unwrap_or_default();let established=snmp.lines().filter(|l|l.starts_with("Tcp:")).nth(1).and_then(|l|l.split_whitespace().nth(9)).and_then(|v|v.parse().ok());System{kernel:read("/proc/sys/kernel/osrelease").unwrap_or_default(),distro,uptime_s:read("/proc/uptime").and_then(|v|v.split_whitespace().next()?.parse().ok()),load:parts.iter().take(3).filter_map(|v|v.parse().ok()).collect(),processes:parts.get(3).and_then(|v|v.split('/').nth(1)).and_then(|v|v.parse().ok()),running:parts.get(3).and_then(|v|v.split('/').next()).and_then(|v|v.parse().ok()),tcp_retransmits_s:delta(prev.retrans,now.retrans,sec),tcp_established:established,forks_s:delta(Some(prev.forks),Some(now.forks),sec),blocked_processes:Some(now.blocked)}}
fn top_processes(previous: &mut HashMap<u32, (u64, u64)>, elapsed: f64) -> Vec<Process> {
    let mut current = HashMap::new();
    let mut processes = Vec::new();
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
            let Some(stat) = read(entry.path().join("stat")) else { continue };
            let Some((head, tail)) = stat.rsplit_once(") ") else { continue };
            let fields: Vec<_> = tail.split_whitespace().collect();
            let Some((start, user, system, rss)) = fields.get(19).and_then(|x| x.parse::<u64>().ok()).zip(fields.get(11).and_then(|x| x.parse::<u64>().ok())).zip(fields.get(12).and_then(|x| x.parse::<u64>().ok())).zip(fields.get(21).and_then(|x| x.parse::<u64>().ok())).map(|(((a,b),c),d)| (a,b,c,d)) else { continue };
            let ticks = user.saturating_add(system);
            let cpu_percent = previous.get(&pid).filter(|(old_start, _)| *old_start == start).and_then(|(_, old)| (hz > 0 && elapsed > 0.0).then(|| ticks.saturating_sub(*old) as f64 * 100.0 / hz as f64 / elapsed));
            current.insert(pid, (start, ticks));
            let name = head.split_once('(').map(|(_, name)| name).unwrap_or("?");
            processes.push(Process { pid, name: name.to_string(), cpu_percent, rss_mib: if page > 0 { rss as f64 * page as f64 / 1_048_576.0 } else { 0.0 } });
        }
    }
    *previous = current;
    let mut memory = processes.clone();
    memory.sort_by(|a,b| b.rss_mib.total_cmp(&a.rss_mib));
    processes.sort_by(|a,b| b.cpu_percent.unwrap_or(0.0).total_cmp(&a.cpu_percent.unwrap_or(0.0)).then_with(|| b.rss_mib.total_cmp(&a.rss_mib)));
    let mut top: Vec<Process> = processes.into_iter().take(8).collect();
    for item in memory.into_iter().take(8) {
        if !top.iter().any(|p| p.pid == item.pid) { top.push(item); }
    }
    top
}
async fn collect(tx:broadcast::Sender<String>){let model=cpu_model();let hostname=read("/proc/sys/kernel/hostname").unwrap_or_default();let paths=discover_sensors();let mut prev=counter_snapshot();let mut previous_time=Instant::now();let mut process_previous=HashMap::new();let mut process_at=Instant::now();let mut process_rows=Vec::new();let mut ticker=interval(Duration::from_millis(TICK_MS));ticker.tick().await;loop{ticker.tick().await;let now=counter_snapshot();let instant=Instant::now();let sec=instant.duration_since(previous_time).as_secs_f64().max(0.001);previous_time=instant;let mut states=HashMap::new();if let (Some(a),Some(b))=(prev.cpu.first(),now.cpu.first()){let dt=b.iter().sum::<u64>().saturating_sub(a.iter().sum::<u64>());for (name,i) in [("user",0),("nice",1),("system",2),("idle",3),("iowait",4),("irq",5),("softirq",6),("steal",7)]{if let Some(v)=pct(at(b,i).and_then(|current|at(a,i).and_then(|previous|current.checked_sub(previous))),Some(dt)){states.insert(name.into(),v);}}}let cpu=Cpu{model:model.clone(),usage_percent:prev.cpu.first().zip(now.cpu.first()).and_then(|(a,b)|cpu_usage(a,b)),per_core_percent:now.cpu.iter().skip(1).zip(prev.cpu.iter().skip(1)).map(|(b,a)|cpu_usage(a,b)).collect(),clock_mhz:cpu_clocks(now.cpu.len().saturating_sub(1)),states_percent:states,context_switches_s:delta(Some(prev.ctxt),Some(now.ctxt),sec),interrupts_s:delta(Some(prev.intr),Some(now.intr),sec)};if process_at.elapsed() >= Duration::from_secs(5) { let elapsed=process_at.elapsed().as_secs_f64(); process_rows=top_processes(&mut process_previous,elapsed); process_at=Instant::now(); }let snapshot=Snapshot{version:"2.7",timestamp:SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs()).unwrap_or(0),hostname:hostname.clone(),cpu,memory:memory(&now,&prev,sec),filesystems:mounted(),block:blocks(&now,&prev,sec),network:networks(&now,&prev,sec),sensors:sensors(&paths),gpu:gpus(),batteries:batteries(),pressure:Pressure{cpu:psi("cpu"),memory:psi("memory"),io:psi("io")},system:system(&now,&prev,sec),top_processes:process_rows.clone()};prev=now;if let Ok(json)=serde_json::to_string(&snapshot){let _=tx.send(json);}}}

/// Loop de coleta para a janela nativa: armazena o último snapshot em um RwLock.
/// JSON é mantido como contrato para esta iteração da GUI.
pub fn start() -> Arc<RwLock<Option<serde_json::Value>>> {
    let latest = Arc::new(RwLock::new(None));
    let slot = Arc::clone(&latest);
    std::thread::Builder::new().name("hw-collector".into()).spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_time().build() {
            Ok(rt) => rt,
            Err(error) => { eprintln!("Falha ao iniciar coletor: {error}"); return; }
        };
        rt.block_on(async move {
            let (tx, _) = broadcast::channel(2);
            let mut rx = tx.subscribe();
            tokio::spawn(collect(tx));
            while let Ok(json) = rx.recv().await {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Ok(mut guard) = slot.write() { *guard = Some(value); }
                }
            }
        });
    }).expect("Falha ao iniciar thread de coleta");
    latest
}

/// Ponto de entrada para o servidor web: recebe um Sender externo e dispara o loop
/// de coleta em background (tokio::spawn), sem duplicar nenhuma lógica de leitura.
/// Requer um runtime Tokio já ativo no contexto do chamador.
#[allow(dead_code)]
pub fn start_broadcast(tx: std::sync::Arc<broadcast::Sender<String>>) {
    tokio::spawn(collect((*tx).clone()));
}
