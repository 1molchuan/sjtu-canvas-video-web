import { PublicApiError } from "../api/client";
import type { LoginEvent } from "../api/schemas";

type VersionedState = { generation: number };

export type LoginState =
  | ({ type: "idle" } & VersionedState)
  | ({ type: "starting" } & VersionedState)
  | ({ type: "waiting"; qrUrl: string } & VersionedState)
  | ({ type: "scanned" } & VersionedState)
  | ({ type: "authenticating" } & VersionedState)
  | ({ type: "completing-session" } & VersionedState)
  | ({ type: "authenticated" } & VersionedState)
  | ({ type: "rejected" } & VersionedState)
  | ({ type: "expired" } & VersionedState)
  | ({ type: "error"; error: PublicApiError } & VersionedState);

export type LoginAction =
  | { type: "start"; generation: number }
  | { type: "event"; generation: number; event: LoginEvent }
  | { type: "complete"; generation: number }
  | { type: "fail"; generation: number; error: PublicApiError }
  | { type: "reset"; generation: number };

export const initialLoginState: LoginState = { type: "idle", generation: 0 };

export function reduceLoginState(state: LoginState, action: LoginAction): LoginState {
  if (action.type === "start") {
    return { type: "starting", generation: action.generation };
  }
  if (action.type === "reset") {
    return { type: "idle", generation: action.generation };
  }
  if (action.generation !== state.generation || isTerminal(state)) {
    return state;
  }
  if (action.type === "complete") {
    return { type: "authenticated", generation: action.generation };
  }
  if (action.type === "fail") {
    return { type: "error", generation: action.generation, error: action.error };
  }
  return stateForEvent(action.event, action.generation);
}

export function isLoginActive(state: LoginState): boolean {
  return ["starting", "waiting", "scanned", "authenticating", "completing-session"].includes(
    state.type,
  );
}

function stateForEvent(event: LoginEvent, generation: number): LoginState {
  switch (event.type) {
    case "started":
      return { type: "starting", generation };
    case "qr":
      return { type: "waiting", generation, qrUrl: event.url };
    case "scanned":
      return { type: "scanned", generation };
    case "authenticating":
      return { type: "authenticating", generation };
    case "authenticated":
      return { type: "completing-session", generation };
    case "rejected":
      return { type: "rejected", generation };
    case "expired":
      return { type: "expired", generation };
    case "error":
      return {
        type: "error",
        generation,
        error: new PublicApiError({ status: 0, code: event.code, message: event.message }),
      };
  }
}

function isTerminal(state: LoginState): boolean {
  return ["authenticated", "rejected", "expired", "error"].includes(state.type);
}
