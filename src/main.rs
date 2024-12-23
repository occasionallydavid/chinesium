use deku::prelude::*;
use std::net::IpAddr;
use std::net::SocketAddr;
use tokio::net::UdpSocket;


const CMD_1TEG_GET_UDP_INFO: u16 = 0x0b;
const CMD_1TEG_UDP_INFO_RESP: u16 = 0x0c;
const CMD_1TEG_SET_UDP_INFO: u16 = 0x0d;
const CMD_2TEG_GET_STREAM: u16 = 0x01;
const CMD_2TEG_IMAGE_DATA: u16 = 0x03;

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


#[derive(Default, Debug, PartialEq, DekuRead, DekuWrite)]
struct TGetStream {
    header: TTEGHeader, // sig=2teg, cmd=0x01, data_len=0x1c
    unknown_a: u64, // must be 0x1
    unknown_b: u32, // must be 0x1
    unknown_c: u64, // must be 0x8
    unknown_d: u32, // must be 0x0
    unknown_e: u32, // must be 0x1
}


#[derive(Default, Debug, PartialEq, DekuRead, DekuWrite)]
struct TImageData {
    header: TTEGHeader, // sig=2teg, cmd=0x03, data_len=..
    unknown: u64,
    frame_index: u16,
    pkt_index: u16,
    image_data_len: u32,
}


impl TGetStream {
    fn new() -> Self {
        Self {
            header: TTEGHeader::new(2, CMD_2TEG_GET_STREAM, 0x1c),
            unknown_a: 0x1,
            unknown_b: 0x1,
            unknown_c: 0x8,
            unknown_d: 0x0,
            unknown_e: 0x1
        }
    }
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
    ImageData(TImageData),
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


async fn camera_main(
    tx: tokio::sync::broadcast::Sender<Vec<u8>>
)
    -> Result<(), std::io::Error>
{
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: chinesium <cam_ip_addr>");
        std::process::exit(1);
    }

    let dst_addr = args[1].as_str();
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    sock.connect((dst_addr, 10104)).await?;
    sock.send(&TGetUDPInfoPkt::new().to_bytes()?).await?;

    let mut buf = [0; 4096];

    timeout_ms(1000, sock.recv(&mut buf)).await??;

    let udp_info: TGetUDPInfo = from_bytes(&buf)?;
    eprintln!("{:?}", udp_info);

    sock.connect((dst_addr, udp_info.udp_info.udp_port)).await?;

    let my_addr = sock.local_addr()?;
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?).await?;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?).await?;

    let timage_data_len = sizeof::<TImageData>();

    let mut start_time = 0;
    let mut frames_received = 0;
    let mut frames_dropped = 0;

    let mut builder = FrameBuilder::new();
    let mut last_get_stream = 0;

    loop {
        if (now() - last_get_stream) > 2000 {
            eprintln!("Send heartbeat");
            sock.send(&TGetStream::new().to_bytes()?).await?;
            //std::thread::sleep(std::time::Duration::from_millis(20));
            sock.send(&TGetStream::new().to_bytes()?).await?;
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
                eprintln!("Recv Video frame={}, pkt={}, len={}",
                         idata.frame_index,
                         idata.pkt_index,
                         idata.image_data_len);

                if idata.pkt_index == 0 {
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
                            match tx.send(v) {
                                Ok(count) => eprintln!("published frame to {} subscribers", count),
                                Err(e) => eprintln!("publish fail: {:?}", e.to_string())
                            }
                        }
                    }
                }

                let start = (idata.header.data_len as usize - 16 - idata.image_data_len as usize) + timage_data_len;
                let rem = &buf[start..start+idata.image_data_len as usize];
                builder.add_piece(idata.pkt_index, rem);
            },
        }
    }
}


async fn web_main(tx: tokio::sync::broadcast::Sender<Vec<u8>>)
    -> Result<(), std::io::Error>
{
    use axum::body::Body;
    use tokio_stream::wrappers::BroadcastStream;

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async {
            axum::response::Html(include_str!("index.html"))
        }))
        .route("/cam", axum::routing::get(|| async move {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "video/x-motion-jpeg")
                .body(Body::from_stream(BroadcastStream::new(tx.subscribe())))
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
    let (tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(40);

    let cam_main = tokio::spawn(camera_main(tx.clone()));
    let web_main = tokio::spawn(web_main(tx.clone()));
    cam_main.await?.unwrap();
    web_main.await?
}
