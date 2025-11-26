// Reexport the native module. On web, it will be resolved to VideoStreamModule.web.ts
// and on native platforms to VideoStreamModule.ts
export { default } from './src/VideoStreamModule';
export { default as VideoStreamView } from './src/VideoStreamView';
export * from  './src/VideoStream.types';
