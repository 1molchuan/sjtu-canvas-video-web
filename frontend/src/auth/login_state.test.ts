import { describe, expect, it } from "vitest";

import { PublicApiError } from "../api/client";
import { initialLoginState, reduceLoginState } from "./login_state";

describe("login state machine", () => {
  it("maps the successful SSE sequence to session completion", () => {
    let state = reduceLoginState(initialLoginState, { type: "start", generation: 1 });
    state = reduceLoginState(state, {
      type: "event",
      generation: 1,
      event: { type: "qr", url: "https://qr.example.test/signed" },
    });
    expect(state).toMatchObject({ type: "waiting", generation: 1 });

    state = reduceLoginState(state, {
      type: "event",
      generation: 1,
      event: { type: "scanned" },
    });
    expect(state.type).toBe("scanned");

    state = reduceLoginState(state, {
      type: "event",
      generation: 1,
      event: { type: "authenticating" },
    });
    state = reduceLoginState(state, {
      type: "event",
      generation: 1,
      event: { type: "authenticated" },
    });
    expect(state.type).toBe("completing-session");
  });

  it("ignores a late event from an older pending login", () => {
    const current = reduceLoginState(initialLoginState, { type: "start", generation: 2 });
    const next = reduceLoginState(current, {
      type: "event",
      generation: 1,
      event: { type: "authenticated" },
    });
    expect(next).toBe(current);
  });

  it("keeps rejected, expired, and public errors distinct", () => {
    const started = reduceLoginState(initialLoginState, { type: "start", generation: 3 });
    expect(
      reduceLoginState(started, {
        type: "event",
        generation: 3,
        event: { type: "rejected" },
      }).type,
    ).toBe("rejected");
    expect(
      reduceLoginState(started, {
        type: "event",
        generation: 3,
        event: { type: "expired" },
      }).type,
    ).toBe("expired");

    const error = new PublicApiError({
      status: 502,
      code: "UPSTREAM_UNAVAILABLE",
      message: "登录失败。",
    });
    expect(
      reduceLoginState(started, { type: "fail", generation: 3, error }),
    ).toMatchObject({ type: "error", error });
  });
});
