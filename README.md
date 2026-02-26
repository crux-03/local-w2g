# Local Watch2Gether
local-w2g is a simple approach to watching videos together with friends. It is currently in an early state, so some things may not work as expected. local-w2g is a simple server that sends events from host to other clients, such as pause, play and seek events. When the client recieves the event, they send an event to a local mpv instance through their JSON IPC. This application **does not** perform any on-the-fly synchronization like traditional w2g does. It only sends simple events when the host pauses, plays or seeks the timeline of the video.

## Recommended Hosting
I found that using Cloudflare tunnels through Cloudflared works really well. This approach requires a domain, however. One thing to look out for is that uploading videos needs to be done locally and not through the tunnel, as Cloudflare will reject any files over 100MB. To get around this, the host can connect to the server through localhost, while participants connect through the exposed url.


## Legal Notice

This software is intended for lawful use only.

The author does not endorse or encourage the use of this software for copyright infringement or illegal file sharing.

Users are solely responsible for complying with applicable laws in their jurisdiction.

The author assumes no liability for misuse of this software.
