import { z } from "zod";

const strictObject = z.strictObject;

export const apiErrorEnvelopeSchema = strictObject({
  error: strictObject({
    code: z.string().min(1),
    message: z.string().min(1),
    request_id: z.string().min(1).optional(),
  }),
});

const sessionUserSchema = strictObject({
  display_label: z.string().min(1),
  identity_source: z.enum(["my_sjtu", "canvas"]),
});

export const sessionResponseSchema = z.discriminatedUnion("authenticated", [
  strictObject({ authenticated: z.literal(false) }),
  strictObject({
    authenticated: z.literal(true),
    user: sessionUserSchema,
    csrf_token: z.string().min(1),
    expires_at: z.iso.datetime({ offset: true }),
  }),
]);

export const qrStartResponseSchema = strictObject({
  pending_id: z.string().min(1),
  events_url: z.string().startsWith("/api/auth/qr/events/"),
  expires_in_seconds: z.number().int().positive(),
});

const loginEventBaseSchemas = [
  strictObject({ type: z.literal("started") }),
  strictObject({ type: z.literal("qr"), url: z.url() }),
  strictObject({ type: z.literal("scanned") }),
  strictObject({ type: z.literal("authenticating") }),
  strictObject({ type: z.literal("authenticated") }),
  strictObject({ type: z.literal("rejected") }),
  strictObject({ type: z.literal("expired") }),
] as const;

export const loginEventSchema = z.discriminatedUnion("type", [
  ...loginEventBaseSchemas,
  strictObject({
    type: z.literal("error"),
    code: z.string().min(1),
    message: z.string().min(1),
  }),
]);

const courseSchema = strictObject({
  id: z.string().min(1),
  name: z.string(),
  course_code: z.string().nullable(),
  term_name: z.string().nullable(),
});

export const coursesResponseSchema = strictObject({
  courses: z.array(courseSchema),
});

const videoSchema = strictObject({
  id: z.string().min(1),
  name: z.string(),
  started_at: z.string().nullable(),
});

export const videosResponseSchema = strictObject({
  videos: z.array(videoSchema),
});

const trackSchema = strictObject({
  id: z.string().min(1),
  kind: z.enum(["camera", "screen", "mixed", "unknown"]),
  suggested_filename: z.string().min(1),
});

export const videoDetailResponseSchema = strictObject({
  video: strictObject({
    id: z.string().min(1),
    name: z.string(),
    tracks: z.array(trackSchema),
  }),
});

export const ticketResponseSchema = strictObject({
  download_url: z.string().startsWith("/api/download/"),
  expires_in_seconds: z.number().int().positive(),
});

export type ApiErrorEnvelope = z.infer<typeof apiErrorEnvelopeSchema>;
export type SessionResponse = z.infer<typeof sessionResponseSchema>;
export type QrStartResponse = z.infer<typeof qrStartResponseSchema>;
export type LoginEvent = z.infer<typeof loginEventSchema>;
export type CoursesResponse = z.infer<typeof coursesResponseSchema>;
export type Course = CoursesResponse["courses"][number];
export type VideosResponse = z.infer<typeof videosResponseSchema>;
export type Video = VideosResponse["videos"][number];
export type VideoDetailResponse = z.infer<typeof videoDetailResponseSchema>;
export type VideoTrack = VideoDetailResponse["video"]["tracks"][number];
export type TicketResponse = z.infer<typeof ticketResponseSchema>;
