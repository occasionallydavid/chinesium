use deku::prelude::*;
use std::time::Duration;
use std::net::Ipv4Addr;
use std::io::Write;

use std::net::IpAddr;
use std::net::UdpSocket;
use std::net::SocketAddr;

const CMD_1TEG_GET_UDP_INFO: u16 = 0x0b;
const CMD_1TEG_SET_UDP_INFO: u16 = 0x0d;
const CMD_2TEG_GET_STREAM: u16 = 0x01;
const CMD_2TEG_IMAGE_DATA: u16 = 0x03;

const TTEG_CMD_EXTERNAL_IPS_UPDATE_RESPONSE: u8 = 0x01;
//const TTEG_CMD_GET_UDP_INFO: u16 = 0x0c;


fn sizeof<T>() -> usize
    where T: Default + deku::DekuContainerWrite
{
    T::default().to_bytes().unwrap().len()
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

    fn as_tuple(&self) -> (u8, u16, u16) {
        (self.version(), self.cmd, self.data_len)
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


fn hex(buf: &[u8]) -> String
{
    let mut out = Vec::new();
    hxdmp::hexdump(buf, &mut out);
    String::from_utf8_lossy(&out).to_string()
}


fn main() -> Result<(), std::io::Error> {
    let args: Vec<_> = std::env::args().collect();
    let dst_addr = args[1].as_str();

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.connect((dst_addr, 10104))?;
    sock.send(&TGetUDPInfoPkt::new().to_bytes()?)?;

    let mut buf = [0; 4096];

    sock.set_read_timeout(Some(Duration::from_millis(1000)))?;
    sock.recv(&mut buf)?;

    let (_, udp_info) = TGetUDPInfo::from_bytes((&buf, 0))?;
    eprintln!("{:?}", udp_info);

    sock.set_read_timeout(Some(Duration::from_millis(100)))?;
    sock.connect((dst_addr, udp_info.udp_info.udp_port))?;

    let my_addr = sock.local_addr()?;
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?)?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    sock.send(&TSetUDPInfo::new(&my_addr).to_bytes()?)?;

    let TIMAGE_DATA_LEN = sizeof::<TImageData>();

    let mut start_time = 0;
    let mut frames_received = 0;

    let mut frame_buf_cohesive = false;
    let mut frame_buf_last_idx = 0;
    let mut frame_buf = Vec::new();
    let mut last_get_stream = 0;

    loop {
        if (now() - last_get_stream) > 2000 {
            eprintln!("HEARTBEAT");
            sock.send(&TGetStream::new().to_bytes()?)?;
            //std::thread::sleep(std::time::Duration::from_millis(20));
            sock.send(&TGetStream::new().to_bytes()?)?;
            last_get_stream = now();
        }

        match sock.recv(&mut buf) {
            Ok(len) => {
                let (_, header) = TTEGHeader::from_bytes((&buf, 0))?;
                match header.as_tuple() {
                    (1, 14, _) => {
                        eprintln!("received (1, 14)");
                    },
                    (2, 1, _) => {
                        eprintln!("received (1, 1)");
                    },
                    (2, 2, _) => {
                        eprintln!("received (1, 2)");
                    },
                    (2, CMD_2TEG_IMAGE_DATA, _) => {
                        let ((rem, _), idata) = TImageData::from_bytes((&buf, 0))?;
                        eprintln!("Video: frame={}, pkt={}, len={}",
                                 idata.frame_index,
                                 idata.pkt_index,
                                 idata.image_data_len);

                        if idata.pkt_index == 0 {
                            if frame_buf_cohesive {
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
                                    eprintln!("EMIT, fps = {}", frames_received as f64 / secs);
                                }
                                std::io::stdout().write_all(&frame_buf)?;
                                std::io::stdout().flush()?;
                            }
                            frame_buf.clear();
                            frame_buf_cohesive = true;
                        }
                        if frame_buf.len() > 0 {
                            if frame_buf_last_idx != (idata.pkt_index - 1) {
                                eprintln!("dropped frame");
                                frame_buf_cohesive = false;
                            }
                        }
                        let start = (idata.header.data_len as usize - 16 - idata.image_data_len as usize) + TIMAGE_DATA_LEN;
                        let rem = &buf[start..start+idata.image_data_len as usize];

                        frame_buf.extend_from_slice(rem);
                        frame_buf_last_idx = idata.pkt_index;
                    },
                    _ => {
                        eprintln!("{:?}", header);
                        eprintln!("{}", hex(&buf[..len]));
                    }
                }
            },
            Err(e) => {
                //return Err(e);
                //eprintln!("sleep");
                //std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    Ok(())
}
