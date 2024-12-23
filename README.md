# Chinesium

This program implements the minimum necessary to output MJPEG from a family of
ultra shady / cheap cameras.


### Typical Product

![Mini Wireless Wifi IP Camera Hidden Spy HD 1080P Home Security Night Vision Cam](images/typical.png)

Similar devices are listed on [eBay](https://www.ebay.co.uk/sch/i.html?_from=R40&_trksid=p2334524.m570.l1313&_nkw=%22iwfcam%22&_sacat=0&LH_TitleDesc=1&_odkw=%22iwfcam%22&_osacat=See-All-Categories) and
[AliExpress](https://vi.aliexpress.com/w/wholesale-%22iwfcam%22.html?spm=a2g0o.home.search.0)
as using the **IWFCam** Android app (`com.g_zhang.mywificam`). Note IWFCam may
support several camera protocols.

This device is characterized by:

* DHCP requests with the hostname `rtthread`
* Client->device UDP broadcasts on port 10104 / 255.255.255.255 for discovery
* Device->mothership UDP traffic on ports 10101, 10102
* Initial client->device communication on UDP ports 10104, 17900
* UDP payloads prefixed with `0TEG`, `1TEG`, `2TEG` signature bytes
* UDP payload references to `cloud.ismartol.com`
* DNS lookups for `cloud.ismartol.com`, `esn-cam.oss-cn-qingdao.aliyuncs.com`
* Lots of UDP traffic to Chinese IP addresses


### Requirements

* Rust compiler


### Intended Usage

1. Factory reset the device
2. Use IWFCam to connect it to a WiFi access point, preferably with no
   Internet access
3. Do not set a password in IWFCam
4. Find the MAC address from your AP and assign it a static IP address, like `10.10.1.30`
5. `cargo run 10.10.1.30 | ffplay -probesize 32 -f mjpeg -`


## Notes

* The program will fail if it is stopped and immediately restarted. Wait a few
  seconds for the device to timeout sending video to the old UDP port before
  restarting

* The camera has 640x480 resolution, not 1080p

* The UDP protocol has no support for retransmission, a single dropped packet
  is enough to cause a dropped video frame

* The device struggles to process UDP messages and seems to require duplicate
  transmissions. IWFCam retransmits some messages 5 times or more. This program
  only retransmits messages twice which may not always be sufficient.

* No support for audio or device controls (yet)


## Warning

These devices and their official app quietly speak UDP to some cloud
mothership. The protocol includes the ability to enumerate local WiFi networks,
which is enough to implement geolocation. Do not connect them to an AP with
Internet access.


### See Also

* https://github.com/fersatgit/IWFCam/ -- Delphi client
* http://www.iwfcam.com/ -- official app

