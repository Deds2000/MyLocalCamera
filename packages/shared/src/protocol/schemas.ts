/** Schemas zod das mensagens JSON do protocolo (paridade com `crates/lc-protocol`).
 *
 * - Campos desconhecidos são ignorados (padrão do `z.object`).
 * - Campos opcionais usam `.default()` (mesmo efeito do `#[serde(default)]`).
 * - `reason` desconhecido vira `"other"`.
 */
import { z } from "zod";
import { REASONS } from "./constants";

const u16 = z.number().int().min(0).max(65535);
const u32 = z.number().int().min(0).max(4294967295);

/** Motivo de recusa (WELCOME) / término (BYE); valor desconhecido vira `"other"`. */
export const reasonSchema = z.string().transform((v) =>
  (REASONS as readonly string[]).includes(v) ? (v as (typeof REASONS)[number]) : "other",
);

export type Reason = (typeof REASONS)[number] | "other";

export const protocolVersionSchema = z.object({
  major: u16,
  minor: u16,
});

export const appInfoSchema = z.object({
  version: z.string(),
  platform: z.string(),
});

export const deviceInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  model: z.string().default(""),
  androidApi: u32.default(0),
});

/** Autenticação no HELLO: `{kind:"pair", token}` ou `{kind:"device", deviceId, key}`. */
export const authSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("pair"), token: z.string() }),
  z.object({ kind: z.literal("device"), deviceId: z.string(), key: z.string() }),
]);

export const cameraModeSchema = z.object({
  w: u32,
  h: u32,
  fps: z.array(u32),
});

export const cameraCapsSchema = z.object({
  id: z.string(),
  facing: z.string(),
  modes: z.array(cameraModeSchema),
});

export const capsSchema = z.object({
  cameras: z.array(cameraCapsSchema).default([]),
  concurrent: z.boolean().default(false),
  concurrentModes: z.array(cameraModeSchema).default([]),
  audio: z.boolean().default(false),
  codecs: z.array(z.string()).default([]),
});

export const helloSchema = z.object({
  protocol: protocolVersionSchema,
  app: appInfoSchema,
  device: deviceInfoSchema,
  auth: authSchema,
  caps: capsSchema.default({
    cameras: [],
    concurrent: false,
    concurrentModes: [],
    audio: false,
    codecs: [],
  }),
});

export const pcInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  appVersion: z.string(),
});

export const welcomeSchema = z.object({
  accepted: z.boolean(),
  protocol: protocolVersionSchema.optional(),
  sessionId: z.string().optional(),
  audioTicket: z.string().optional(),
  deviceKey: z.string().optional(),
  pc: pcInfoSchema.optional(),
  reason: reasonSchema.optional(),
  minSupported: protocolVersionSchema.optional(),
});

export const audioHelloSchema = z.object({
  sessionId: z.string(),
  ticket: z.string(),
});

export const pendingSchema = z.object({
  timeoutS: u32,
});

export const audioTicketSchema = z.object({
  ticket: z.string(),
});

export const streamConfigSchema = z.object({
  stream: z.number().int().min(0).max(255),
  width: u32,
  height: u32,
  fps: u32,
  rotationHint: u16,
  facing: z.string(),
  profile: z.string().optional(),
});

export const setItemSchema = z.object({
  key: z.string(),
  value: z.unknown(),
});

/** SET aceita um item `{key,value}` ou uma lista deles. */
export const setPayloadSchema = z.union([z.array(setItemSchema), setItemSchema]);

export const sidesSchema = <T extends z.ZodType>(item: T) =>
  z.object({ back: item, front: item });

export const cameraStateSchema = z.object({
  mode: z.string(),
  active: z.string(),
});

export const videoStateSchema = z.object({
  width: u32,
  height: u32,
  fps: u32,
  bitrateKbps: u32,
});

export const presentationSchema = z.object({
  mirror: sidesSchema(z.boolean()),
  rotation: sidesSchema(u16),
});

export const audioStateSchema = z.object({
  enabled: z.boolean(),
  gain: z.number(),
});

export const stateSchema = z.object({
  rev: z.number().int().min(0),
  camera: cameraStateSchema.default({ mode: "single", active: "back" }),
  video: videoStateSchema.default({ width: 0, height: 0, fps: 0, bitrateKbps: 0 }),
  presentation: presentationSchema.default({
    mirror: { back: false, front: false },
    rotation: { back: 0, front: 0 },
  }),
  audio: audioStateSchema.default({ enabled: false, gain: 1 }),
});

export const statsSchema = z.object({
  recvFps: z.number(),
  dropped: u32,
  queueMs: u32,
});

export const byeSchema = z.object({
  reason: z.string(),
});

/* Tipos inferidos (paridade com `z.infer` do cartão). */
export type ProtocolVersion = z.infer<typeof protocolVersionSchema>;
export type AppInfo = z.infer<typeof appInfoSchema>;
export type DeviceInfo = z.infer<typeof deviceInfoSchema>;
export type Auth = z.infer<typeof authSchema>;
export type CameraMode = z.infer<typeof cameraModeSchema>;
export type CameraCaps = z.infer<typeof cameraCapsSchema>;
export type Caps = z.infer<typeof capsSchema>;
export type Hello = z.infer<typeof helloSchema>;
export type PcInfo = z.infer<typeof pcInfoSchema>;
export type Welcome = z.infer<typeof welcomeSchema>;
export type AudioHello = z.infer<typeof audioHelloSchema>;
export type Pending = z.infer<typeof pendingSchema>;
export type AudioTicket = z.infer<typeof audioTicketSchema>;
export type StreamConfig = z.infer<typeof streamConfigSchema>;
export type SetItem = z.infer<typeof setItemSchema>;
export type SetPayload = z.infer<typeof setPayloadSchema>;
export type CameraState = z.infer<typeof cameraStateSchema>;
export type VideoState = z.infer<typeof videoStateSchema>;
export type Presentation = z.infer<typeof presentationSchema>;
export type AudioState = z.infer<typeof audioStateSchema>;
export type State = z.infer<typeof stateSchema>;
export type Stats = z.infer<typeof statsSchema>;
export type Bye = z.infer<typeof byeSchema>;
