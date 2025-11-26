import { requireNativeView } from 'expo';
import * as React from 'react';

import { VideoStreamViewProps } from './VideoStream.types';

const NativeView: React.ComponentType<VideoStreamViewProps> =
  requireNativeView('VideoStream');

export default function VideoStreamView(props: VideoStreamViewProps) {
  return <NativeView {...props} />;
}
