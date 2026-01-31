# Local Watch2Gether
VIBE-CODE WARNING!! - This was put together over the course of a few days almost exclusively using Claude. Use at your own risk.

This project is a simplified approach to Watch2Gether and completely self-hostable. It works by using MPV's JSON IPC and a lightweight client to let the owner pause and resume the playback (seeking currently unsupported). The server is also very light, and only redirects messages to other clients. **This application does not perform on-the-fly synchronization**, but rather relies purely on pause and play requests (for now). The owner uploads a video to the server, other clients download it, then the owner starts playback.

In my testing, I have found that the server works well routing through Cloudflare Tunnels, apart from there being a small caveat. When uploading videos, you need to do so locally, as Cloudflare has a 100MB upload limit. Downloading videos still works fine, as those are streamed in smaller chunks.

This project is still in a very early stage, so production use is not recommended. If you have any feedback or want to contribute, feel free. I might work on this from time to time, but it's not my primary focus at the moment. The final plan is to allow native controls via MPV when pausing/resuming/seeking. For now, use the controls in the client.
