import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { extname, resolve, sep } from "node:path";

const HOST = "127.0.0.1";
const PORT = 4173;
const ORIGIN = `http://${HOST}:${String(PORT)}`;
const DIST = resolve(import.meta.dirname, "../dist");
const pending = new Map();
const sessions = new Set();
const tickets = new Map();
let sequence = 0;

const server = createServer((request, response) => {
  void route(request, response).catch(() => {
    if (!response.headersSent) error(response, 500, "INTERNAL", "Fixture request failed.");
    else response.destroy();
  });
});

server.listen(PORT, HOST, () => {
  process.stdout.write(`fixture server listening on ${ORIGIN}\n`);
});

async function route(request, response) {
  const url = new URL(request.url ?? "/", ORIGIN);
  const path = url.pathname;
  if (path === "/api/health") return json(response, 200, { status: "ok" });
  if (path === "/api/auth/qr/start" && request.method === "POST") return startLogin(response);
  if (path.startsWith("/api/auth/qr/events/") && request.method === "GET") {
    return loginEvents(request, response, path.split("/").at(-1) ?? "");
  }
  if (path === "/api/auth/session") return authSession(request, response);
  if (path === "/api/auth/logout" && request.method === "POST") return logout(request, response);
  if (path === "/__fixture/expire" && request.method === "POST") return expire(request, response);
  if (path === "/api/courses") return authenticated(request, response, listCourses);
  if (path === "/api/courses/opaque-course-fail/videos") return courseFailure(response);
  if (path === "/api/courses/opaque-course-success/videos") return authenticated(request, response, listVideos);
  if (path === "/api/courses/opaque-course-success/videos/opaque-video") {
    return authenticated(request, response, videoDetail);
  }
  if (path.endsWith("/ticket") && request.method === "POST") return issueTicket(request, response);
  if (path.startsWith("/api/download/")) return download(request, response, path);
  if (path.startsWith("/api/")) return error(response, 404, "NOT_FOUND", "Fixture API not found.");
  return staticFile(response, path);
}

function startLogin(response) {
  sequence += 1;
  const id = `fixture-pending-${String(sequence)}`;
  pending.set(id, false);
  response.setHeader("Set-Cookie", `fixture_pending=${id}; Path=/; HttpOnly; SameSite=Lax`);
  json(response, 200, {
    pending_id: id,
    events_url: `/api/auth/qr/events/${id}`,
    expires_in_seconds: 300,
  });
}

function loginEvents(request, response, id) {
  if (cookies(request).fixture_pending !== id || !pending.has(id)) {
    return error(response, 404, "PENDING_LOGIN_NOT_FOUND", "Login not found.");
  }
  response.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-store",
    Connection: "keep-alive",
  });
  const events = [
    [0, { type: "started" }],
    [100, { type: "qr", url: "https://qr.example.test/fixture" }],
    [800, { type: "scanned" }],
    [1_200, { type: "authenticating" }],
    [1_600, { type: "authenticated" }],
  ];
  const timers = events.map(([delay, event]) => setTimeout(() => {
    if (event.type === "authenticated") pending.set(id, true);
    response.write(`data: ${JSON.stringify(event)}\n\n`);
    if (event.type === "authenticated") response.end();
  }, delay));
  response.on("close", () => timers.forEach(clearTimeout));
}

function authSession(request, response) {
  const cookie = cookies(request);
  if (cookie.fixture_session && sessions.has(cookie.fixture_session)) {
    return json(response, 200, sessionBody());
  }
  const id = cookie.fixture_pending;
  if (!id || pending.get(id) !== true) return json(response, 200, { authenticated: false });
  const session = `fixture-session-${id}`;
  sessions.add(session);
  pending.delete(id);
  response.setHeader("Set-Cookie", [
    `fixture_session=${session}; Path=/; HttpOnly; SameSite=Lax`,
    "fixture_pending=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
  ]);
  return json(response, 200, sessionBody());
}

function sessionBody() {
  return {
    authenticated: true,
    user: { display_label: "已登录用户", identity_source: "canvas" },
    csrf_token: "fixture-csrf-memory-only",
    expires_at: "2030-01-01T00:00:00Z",
  };
}

function listCourses(_request, response) {
  json(response, 200, {
    courses: [
      { id: "opaque-course-fail", name: "暂不可用课程", course_code: "ERR101", term_name: null },
      { id: "opaque-course-success", name: "可用课程", course_code: "OK101", term_name: "测试学期" },
    ],
  });
}

function courseFailure(response) {
  error(response, 502, "UPSTREAM_UNAVAILABLE", "上游服务暂时不可用。", "fixture-request-502");
}

function listVideos(_request, response) {
  json(response, 200, {
    videos: [{ id: "opaque-video", name: "第一讲：课程介绍", started_at: "2030-01-01T09:00:00Z" }],
  });
}

function videoDetail(_request, response) {
  json(response, 200, {
    video: {
      id: "opaque-video",
      name: "第一讲：课程介绍",
      tracks: [
        { id: "opaque-track-one", kind: "unknown", suggested_filename: "lecture-track-1.mp4" },
        { id: "opaque-track-two", kind: "unknown", suggested_filename: "lecture-track-2.mp4" },
      ],
    },
  });
}

function issueTicket(request, response) {
  const session = activeSession(request);
  if (!session) return error(response, 401, "UNAUTHORIZED", "Login required.");
  if (request.headers["x-csrf-token"] !== "fixture-csrf-memory-only") {
    return error(response, 403, "CSRF_REJECTED", "CSRF rejected.");
  }
  sequence += 1;
  const id = `fixture-ticket-${String(sequence)}`;
  tickets.set(id, session);
  json(response, 200, { download_url: `/api/download/${id}`, expires_in_seconds: 60 });
}

function download(request, response, path) {
  const id = path.split("/").at(-1) ?? "";
  const session = activeSession(request);
  if (!session) return error(response, 401, "UNAUTHORIZED", "Login required.");
  if (tickets.get(id) !== session) return error(response, 404, "DOWNLOAD_TICKET_INVALID", "Invalid ticket.");
  const body = Buffer.from("fixture-video");
  response.setHeader("Content-Type", "video/mp4");
  response.setHeader("Content-Disposition", 'attachment; filename="lecture-track-1.mp4"');
  response.setHeader("Cache-Control", "private, no-store");
  response.setHeader("Accept-Ranges", "bytes");
  if (request.headers.range === "bytes=0-0") {
    response.writeHead(206, { "Content-Range": `bytes 0-0/${String(body.length)}`, "Content-Length": "1" });
    return response.end(body.subarray(0, 1));
  }
  response.writeHead(200, { "Content-Length": String(body.length) });
  return response.end(request.method === "HEAD" ? undefined : body);
}

function logout(request, response) {
  const session = activeSession(request);
  if (session) {
    sessions.delete(session);
    for (const [ticket, owner] of tickets) if (owner === session) tickets.delete(ticket);
  }
  response.setHeader("Set-Cookie", "fixture_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
  response.writeHead(204, { "Cache-Control": "no-store" });
  response.end();
}

function expire(request, response) {
  const session = activeSession(request);
  if (session) sessions.delete(session);
  response.writeHead(204);
  response.end();
}

function authenticated(request, response, handler) {
  if (!activeSession(request)) return error(response, 401, "UNAUTHORIZED", "Login required.");
  return handler(request, response);
}

function activeSession(request) {
  const session = cookies(request).fixture_session;
  return session && sessions.has(session) ? session : null;
}

function cookies(request) {
  return Object.fromEntries((request.headers.cookie ?? "").split(";").flatMap((entry) => {
    const [name, ...value] = entry.trim().split("=");
    return name ? [[name, value.join("=")]] : [];
  }));
}

function staticFile(response, pathname) {
  const requested = pathname === "/" ? "/index.html" : pathname;
  const file = resolve(DIST, `.${requested}`);
  const inside = file.startsWith(`${DIST}${sep}`);
  const asset = requested.startsWith("/assets/") || requested === "/favicon.svg";
  if (!inside) return error(response, 404, "NOT_FOUND", "Not found.");
  try {
    const content = readFileSync(file);
    response.writeHead(200, {
      "Content-Type": contentType(file),
      "Cache-Control": requested.startsWith("/assets/") ? "public, max-age=31536000, immutable" : "no-cache",
    });
    response.end(content);
  } catch {
    if (asset) return error(response, 404, "NOT_FOUND", "Not found.");
    return staticFile(response, "/index.html");
  }
}

function contentType(file) {
  return ({ ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml" })[extname(file)] ?? "application/octet-stream";
}

function json(response, status, body) {
  const payload = JSON.stringify(body);
  response.writeHead(status, { "Content-Type": "application/json", "Cache-Control": "no-store" });
  response.end(payload);
}

function error(response, status, code, message, requestId = "fixture-request") {
  json(response, status, { error: { code, message, request_id: requestId } });
}
