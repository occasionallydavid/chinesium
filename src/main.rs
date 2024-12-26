use deku::prelude::*;
use std::net::IpAddr;
use std::net::SocketAddr;
use tokio::net::UdpSocket;


const CMD_1TEG_DEVICE_CLOSED: u16 = 0x10; // data_len = 0x14 all zeroes
const CMD_1TEG_APP_HEARTBEAT: u16 = 0x03;
const CMD_1TEG_GET_UDP_INFO: u16 = 0x0b;
const CMD_1TEG_UDP_INFO_RESP: u16 = 0x0c;
const CMD_1TEG_SET_UDP_INFO: u16 = 0x0d;
const CMD_2TEG_PPP_CONTROL_CMD : u16 = 0x01;
const CMD_2TEG_IMAGE_DATA: u16 = 0x03;
const CMD_2TEG_MAYBE_SOUND: u16 = 0x04;

//const TTEG_CMD_EXTERNAL_IPS_UPDATE_RESPONSE: u8 = 0x01;


fn sizeof<T>() -> usize
    where T: Default + deku::DekuContainerWrite
{
    T::default().to_bytes().unwrap().len()
}


fn from_bytes<'a, T>(buf: &'a [u8]) -> Result<T, std::io::Error>
    where T: deku::DekuContainerRead<'a>
{
    Ok(T::from_bytes((buf, 0))?.1)
}


#[derive(Default, Debug, PartialEq, DekuRead, DekuWrite)]
//#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
// sizeof(TTEGHeader) == 8
pub struct TTEGHeader {
    signature: [u8; 4],
    cmd: u16,
    data_len: u16,
}


impl TTEGHeader {
    fn new(version: u8, cmd: u16, data_len: u16) -> Self {
        Self {
            signature: match version {
                1 => *b"1TEG",
                2 => *b"2TEG",
                _ => panic!(),
            },
            cmd: cmd,
            data_len: data_len,
        }
    }

    fn version(&self) -> u8 {
        match self.signature {
            [0x31, 0x54, 0x45, 0x47] => 1,
            [0x32, 0x54, 0x45, 0x47] => 2,
            _ => panic!()
        }
    }
}


#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct TUDPInfo {
    unknown: [u8; 18],
    #[deku(endian = "big")]
    udp_port: u16,
    ip_addr: [u8; 4],
}


impl TUDPInfo {
    fn new(my_addr: &SocketAddr) -> Self {
        Self {
            unknown: [0; 18],
            udp_port: my_addr.port(),
            ip_addr: match my_addr.ip() {
                IpAddr::V4(v4) => v4.octets(),
                _ => panic!()
            },
        }
    }
}


#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct TGetUDPInfo {
    header: TTEGHeader,
    unknown_1: u32,
    unknown_2: u32,
    udp_info: TUDPInfo,
    cam_name: [u8; 28],
    unknown_3: u64,
}


#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct TSetUDPInfo {
    header: TTEGHeader,
    udp_info: TUDPInfo,
    unknown_2: u8,
    password_hash: [u8; 12],
    align: [u8; 3],
}


impl TSetUDPInfo {
    fn new(my_addr: &SocketAddr) -> Self
    {
        Self {
            header: TTEGHeader::new(1, CMD_1TEG_SET_UDP_INFO, 40),
            udp_info: TUDPInfo::new(my_addr),
            unknown_2: 9,
            password_hash: *b"9e8040834b3a",
            align: [0; 3],
        }
    }
}


#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct TGetUDPInfoPkt {
    header: TTEGHeader, // data_len == 12
    unknown_1: u16, // must be 0x01
    pad: [u8; 10]
}


impl TGetUDPInfoPkt {
    fn new() -> Self {
        Self {
            header: TTEGHeader::new(1, CMD_1TEG_GET_UDP_INFO, 12),
            unknown_1: 0x01,
            pad: [0; 10]
        }
    }
}





// DevP2PSendPPPCtlCmdResp: (2TEG, 0, 4, 

// DeviceRequLiveMediaInfor:        DevP2PSendPPPControlCmd(this,0,1,8,(uchar *)&local_30,0,param_2);
// DeviceCheckHeartBeat:            DevP2PSendPPPControlCmd(this,2,0,8,local_40,0,0);
// DevDo433mOper:                   DevP2PSendPPPControlCmd(this,0xd,0xc,0x1c,(uchar *)&local_58,0,0);
// DevSetWifiConnect:               DevP2PSendPPPControlCmd(this,0xe,3,0x82,(uchar *)param_1,0,0);
// SetP2PWifiAPMode:                DevP2PSendPPPControlCmd(this,0xe,6,0x84,(uchar *)pCVar1,0,0);
// DevSetSDCardConfig;              DevP2PSendPPPControlCmd(this,0xf,1,0x78,(uchar *)(this + 0x1f18),0,0);
// ProcRecvSDCardPlayFrame:         DevP2PSendPPPControlCmd(this,0xf,1,0x78,(uchar *)(this + 0x1f18),0,0);
// ProcRecvSDCardPlayFrame:         DevP2PSendPPPControlCmd(this,0xf,3,0x78,(uchar *)&local_d0,0,0);
// DevSetIRLedCtl:                  DevP2PSendPPPControlCmd(this,0x13,5,0xc,(uchar *)&local_38,0,0);
// DevSetIRLedConfig:               DevP2PSendPPPControlCmd(this,0x13,2,0x28,(uchar *)(this + 0x1fa8),0,0);
// DevSetNTPConfig:                 DevP2PSendPPPControlCmd(this,0x15,1,0xa0,(uchar *)(this + 0x1fd0),0,0);
// DevSetEMailConfig:               DevP2PSendPPPControlCmd(this,0x16,1,0x2f4,(uchar *)(this + 0x2070),0,0);
// DevSetFTPConfig:                 DevP2PSendPPPControlCmd(this,0x17,1,0x144,(uchar *)(this + 0x23ec),0,0);
// DevSetDDNSConfg:                 DevP2PSendPPPControlCmd(this,0x18,1,0x22c,(uchar *)(this + 0x25b8),0,0);
// DevSetAccessPwd:                 DevP2PSendPPPControlCmd(this,0x12,1,0x44,local_a0,0,0);
// DevSetAccessPwd:                 DevP2PSendPPPControlCmd(this,0x19,2,0x34,local_a0,0,0);
// DevSetUserInfor:                 DevP2PSendPPPControlCmd(this,0x19,1,0xf4,(uchar *)pCVar1,0,0);
// DevSetAccessUserDisabled:        DevP2PSendPPPControlCmd(this,0x19,4,8,(uchar *)&local_30,0,0);
// DevOTAUpdateStart:               DevP2PSendPPPControlCmd(this,0x1a,3,0xcc,local_110,0,0);
// DevOTAUpdateGetStatus:           DevP2PSendPPPControlCmd(this,0x1a,3,0xcc,local_100,0,0);
// DevSetDevIPInfor:                DevP2PSendPPPControlCmd(this,0x1a,1,0x84,(uchar *)(this + 0x28d8),0,0);
// DevP2PSendEncodeInfor:           DevP2PSendPPPControlCmd(this,0x10,1,0x14,(uchar *)&local_40,1,0);




#[derive(Default, Debug, PartialEq, DekuRead, DekuWrite)]
struct ControlCommand {
    header: TTEGHeader, // sig=2teg, cmd=0x01, data_len=0x1c
    unknown_a: u64, // must be 0x1
    unknown_b: u32, // must be 0x1
    unknown_c: u64, // must be 0x8
    unknown_d: u32, // must be 0x0
    unknown_e: u32, // must be 0x1
}


impl ControlCommand {
    fn new() -> Self {
        Self {
            header: TTEGHeader::new(2, CMD_2TEG_PPP_CONTROL_CMD, 0x1c),
            unknown_a: 0x1,
            unknown_b: 0x1,
            unknown_c: 0x8,
            unknown_d: 0x0,
            unknown_e: 0x1
        }
    }
}


#[derive(Default, Debug, PartialEq, DekuRead, DekuWrite)]
struct DataFrame {
    header: TTEGHeader, // sig=2teg, cmd=0x03, data_len=..
    unknown_0: u32,  // 01 00 00 00
    unknown_1: u16,  // 01 00
    is_audio: u16,  // 00 00 = video,  01 00 = audio
    frame_index: u16,
    pkt_index: u16,
    image_data_len: u32,
}


fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}


fn hex2(buf: &[u8]) -> String
{
    let mut s = String::new();
    for i in buf {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", i).expect("could not write");
    }
    s
}


struct FrameBuilder {
    pieces: Vec<Option<Vec<u8>>>,
}


impl FrameBuilder {
    fn new() -> Self {
        Self { pieces: Vec::new() }
    }

    fn add_piece(&mut self, pkt_id: u16, buf: &[u8]) {
        if self.pieces.len() <= (pkt_id as usize) {
            self.pieces.resize(pkt_id as usize + 1, None);
        }
        self.pieces[pkt_id as usize] = Some(buf.to_vec());
    }

    fn finalize(&mut self) -> Option<Vec<u8>> {
        let out = match
            self.pieces.iter().any(|ov| matches!(ov, None)) ||
            self.pieces.is_empty()
        {
            true => None,
            false => Some(self.pieces.iter().flatten().flatten().map(|c| *c).collect())
        };
        self.pieces.clear();
        out
    }
}


enum Frame {
    UdpInfoResp(TGetUDPInfo),
    ImageData(DataFrame),
    Unknown(TTEGHeader),
}


fn parse_frame(buf: &[u8]) -> Result<Frame, std::io::Error> {
    let (_, header) = TTEGHeader::from_bytes((buf, 0))?;
    Ok(match (header.version(), header.cmd) {
        (1, CMD_1TEG_UDP_INFO_RESP) => Frame::UdpInfoResp(from_bytes(buf)?),
        (2, CMD_2TEG_IMAGE_DATA) => Frame::ImageData(from_bytes(buf)?),
        _ => Frame::Unknown(header),
    })
}


fn timeout_ms<F>(ms: u64, future: F)
    -> tokio::time::Timeout<F::IntoFuture>
where
    F: std::future::IntoFuture
{
    tokio::time::timeout(
        std::time::Duration::from_millis(ms),
        future
    )
}


//const MAYBE_AUDIO: &[u8] = &hex!("3254454701001400010000000f000000020000000000000000000000");
const MAYBE_AUDIO: &[u8] = &[
    0x32, 0x54, 0x45, 0x47, 0x01, 0x00, 0x1c, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00,
];


async fn camera_main(
    dst_addr: String,
    video_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
    audio_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
)
    -> Result<(), std::io::Error>
{
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    sock.connect((dst_addr.as_str(), 10104)).await?;
    sock.send(&TGetUDPInfoPkt::new().to_bytes()?).await?;

    let mut buf = [0; 4096];

    timeout_ms(1000, sock.recv(&mut buf)).await??;

    let udp_info: TGetUDPInfo = from_bytes(&buf)?;
    eprintln!("{:?}", udp_info);

    sock.connect((dst_addr.as_str(), udp_info.udp_info.udp_port)).await?;

    let my_addr = sock.local_addr()?;
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?).await?;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?).await?;

    let sizeof_data_frame = sizeof::<DataFrame>();

    let mut start_time = 0;
    let mut frames_received = 0;
    let mut frames_dropped = 0;

    let mut builder = FrameBuilder::new();
    let mut last_get_stream = 0;

    loop {
        if (now() - last_get_stream) > 2000 {
            eprintln!("Send heartbeat");
            sock.send(&ControlCommand::new().to_bytes()?).await?;
            //std::thread::sleep(std::time::Duration::from_millis(20));
            sock.send(&ControlCommand::new().to_bytes()?).await?;
            sock.send(&MAYBE_AUDIO).await?;
            sock.send(&MAYBE_AUDIO).await?;
            last_get_stream = now();
        }

        let len = match timeout_ms(100, sock.recv(&mut buf)).await {
            Ok(Ok(len)) => len,
            _ => continue,
        };

        match parse_frame(&buf[..len])? {
            Frame::Unknown(header) => {
                eprintln!("Recv ({}, {}, {}) -> {}",
                          header.version(), header.cmd,
                          header.data_len, hex2(&buf[..len]));
            }
            Frame::UdpInfoResp(_) => {},
            Frame::ImageData(idata) => {
                eprintln!("Recv Video frame={}, pkt={}, len={}, is_audio={}",
                          idata.frame_index,
                          idata.pkt_index,
                          idata.image_data_len,
                          idata.is_audio);

                if idata.pkt_index == 0 {
                    if idata.is_audio == 1 {
                        let start = (idata.header.data_len as usize - 16 - idata.image_data_len as usize) + sizeof_data_frame;
                        let v = &buf[start..start+idata.image_data_len as usize];
                        let vlen = v.len();
                        let subcount = audio_tx.send(v.to_vec()).unwrap_or(0);
                        if subcount > 0 {
                            eprintln!("published {} byte audio frame to {} subscribers", vlen, subcount);
                        }
                        continue;
                    }

                    match builder.finalize() {
                        None => {
                            eprintln!("dropped frame");
                            frames_dropped += 1;
                        },
                        Some(v) => {
                            // emit
                            frames_received += 1;
                            if start_time == 0 {
                                start_time = now();
                            } else {
                                let ms = now() - start_time;
                                let secs = match ms / 1000 {
                                    0 => 1.0,
                                    _ => ms as f64 / 1000.0
                                };
                                eprintln!("EMIT, fps={}, dropped={}, received={}",
                                          (frames_dropped+frames_received) as f64 / secs,
                                          frames_dropped, frames_received);
                            }
                            let vlen = v.len();
                            let subcount = video_tx.send(v).unwrap_or(0);
                            if subcount > 0 {
                                eprintln!("published {} byte video frame to {} subscribers", vlen, subcount);
                            }
                        }
                    }
                }

                let start = (idata.header.data_len as usize - 16 - idata.image_data_len as usize) + sizeof_data_frame;
                let rem = &buf[start..start+idata.image_data_len as usize];
                builder.add_piece(idata.pkt_index, rem);
            },
        }
    }
}


async fn web_main(
    video_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
    audio_tx: tokio::sync::broadcast::Sender<Vec<u8>>
)
    -> Result<(), std::io::Error>
{
    use axum::body::Body;
    use tokio_stream::wrappers::BroadcastStream;

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async {
            axum::response::Html(include_str!("index.html"))
        }))
        .route("/audio", axum::routing::get(|| async move {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "audio/x-ima-adpcm")
                .body(Body::from_stream(BroadcastStream::new(audio_tx.subscribe())))
                .unwrap()
        }))
        .route("/cam", axum::routing::get(|| async move {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "video/x-motion-jpeg")
                .body(Body::from_stream(BroadcastStream::new(video_tx.subscribe())))
                .unwrap()
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
    Ok(())
}


#[tokio::main(flavor="current_thread")]
async fn main() -> Result<(), std::io::Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: chinesium <cam_ip_addr>");
        std::process::exit(1);
    }

    let dst_addr = args[1].as_str();
    let (video_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(40);
    let (audio_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(40);

    let cam_main = tokio::spawn(
        camera_main(dst_addr.to_string(),
                    video_tx.clone(),
                    audio_tx.clone()));
    let web_main = tokio::spawn(
        web_main(video_tx.clone(), audio_tx.clone())
    );
    cam_main.await?.unwrap();
    web_main.await?
}
