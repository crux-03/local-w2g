export interface TrackInfo {
  index: number,
  codec: string,
  language?: string,
  title?: string,
  is_default: boolean
}

export interface MediaProbe {
  audio_tracks: TrackInfo[]
  subtitle_tracks: TrackInfo[]
}