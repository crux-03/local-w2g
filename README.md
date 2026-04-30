# Local Watch2Gether

**local-w2g** is an opinionated implementation of the popular Watch2Gether service. It trades feature complexity for lower bandwidth and processing requirements. It's designed for friend groups who need an easy-to-set-up, private-ish solution for synchronized media playback.

<img width="1306" height="889" alt="image" src="https://github.com/user-attachments/assets/a2d44388-6847-40b5-8290-c511829532ad" />

## Design Philosophy
Local Watch2Gether is designed to be light and relatively stateless. It's built around the principle of sparse update events rather than dense on-the-fly synchronization.

In practice, this means each client downloads the media file once from your server, then plays it locally. Your server doesn't stream anything — it just hosts the file. After that, clients stay in sync by exchanging small, infrequent control events. The tradeoff is slightly higher latency between clients (not noticable in practice), but in return you get full-quality playback without paying for a beefy media server.

We support the following control events:
- Play
- Pause
- Resume
- Seek (forward/backward)
- Resync (force re-synchronization of clients)

Since these events are only sent when needed, your server stays mostly idle and the clients do most of the work.
## Features
Local Watch2Gether ships with a limited but powerful feature set to help streamline your sessions.

### Permission System
One issue with traditional Watch2Gether is that you have little control over who can pause, edit the playlist, and so on.

We fix this with a thin permission system covering playback and a few other actions. It makes your life as a host easier without any noticeable performance hit.

### Chat Interface
A must-have for remote sessions. I spent a lot of time on the chat interface to make sure it conveys information in an easily digestible way. It supports normal chat messages, system logs, and dynamic widgets — things like download/upload progress and resync results.

### Playlist Editing
We built a small system for editing media entries. The headline feature: changing the default audio and subtitle tracks. No more jumpscares when playing media with terrible dubs :)

## Legal Notice

This software is intended for lawful use only.

The author does not endorse or encourage the use of this software for copyright infringement or illegal file sharing.

Users are solely responsible for complying with applicable laws in their jurisdiction.

The author assumes no liability for misuse of this software.
