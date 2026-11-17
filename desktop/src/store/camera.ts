import type { AppSet } from "./types";

/** The STATE payload mirrors the phone's protocol without interpreting its sub-objects. */
export interface CameraStateEvent {
  rev: number;
  camera: Record<string, unknown> | null;
  video: Record<string, unknown> | null;
  presentation: Record<string, unknown> | null;
  audio: Record<string, unknown> | null;
}

export interface CameraSnapshot extends CameraStateEvent {
  pending: boolean;
}

export interface CameraSlice {
  cameraState: CameraSnapshot | null;
  applyCameraState: (event: CameraStateEvent) => void;
}

export function createCameraSlice(set: AppSet): CameraSlice {
  return {
    cameraState: null,
    applyCameraState: (event) => set({ cameraState: { ...event, pending: false } }),
  };
}
