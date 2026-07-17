import { useCallback, useEffect, useReducer, useRef } from "react";

import { PublicApiError } from "../api/client";
import {
  loginEventSchema,
  type LoginEvent,
  type QrStartResponse,
  type SessionResponse,
} from "../api/schemas";
import { initialLoginState, isLoginActive, reduceLoginState } from "./login_state";

export type LoginEventStream = {
  onmessage: ((event: MessageEvent<string>) => void) | null;
  onerror: ((event: Event) => void) | null;
  close: () => void;
};

export type QrLoginDependencies = {
  startLogin: () => Promise<QrStartResponse>;
  claimSession: () => Promise<SessionResponse>;
  openEvents: (url: string) => LoginEventStream;
};

type UseQrLoginOptions = {
  deps: QrLoginDependencies;
  onAuthenticated: (session: Extract<SessionResponse, { authenticated: true }>) => void;
};

export function useQrLogin(options: UseQrLoginOptions) {
  const [state, dispatch] = useReducer(reduceLoginState, initialLoginState);
  const generationRef = useRef(0);
  const activeRef = useRef(false);
  const sourceRef = useRef<LoginEventStream | null>(null);

  const closeEvents = useCallback(() => {
    sourceRef.current?.close();
    sourceRef.current = null;
  }, []);

  const fail = useCallback(
    (generation: number, error: unknown) => {
      if (generation !== generationRef.current) return;
      activeRef.current = false;
      closeEvents();
      dispatch({ type: "fail", generation, error: toPublicError(error) });
    },
    [closeEvents],
  );

  const completeSession = useCallback(
    async (generation: number) => {
      try {
        const session = await options.deps.claimSession();
        if (!session.authenticated) {
          throw sessionClaimError();
        }
        if (generation !== generationRef.current) return;
        activeRef.current = false;
        dispatch({ type: "complete", generation });
        options.onAuthenticated(session);
      } catch (error) {
        fail(generation, error);
      }
    },
    [fail, options],
  );

  const connect = useCallback(
    (start: QrStartResponse, generation: number) => {
      if (generation !== generationRef.current) return;
      const source = options.deps.openEvents(start.events_url);
      sourceRef.current = source;
      source.onmessage = (message) => {
        if (generation !== generationRef.current) return;
        const event = parseLoginEvent(message.data);
        if (event === null) {
          fail(generation, invalidEventError());
          return;
        }
        dispatch({ type: "event", generation, event });
        if (event.type === "authenticated") {
          closeEvents();
          void completeSession(generation);
        } else if (["rejected", "expired", "error"].includes(event.type)) {
          activeRef.current = false;
          closeEvents();
        }
      };
      source.onerror = () => fail(generation, sseError());
    },
    [closeEvents, completeSession, fail, options.deps],
  );

  const start = useCallback(async () => {
    if (activeRef.current) return;
    activeRef.current = true;
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    dispatch({ type: "start", generation });
    try {
      connect(await options.deps.startLogin(), generation);
    } catch (error) {
      fail(generation, error);
    }
  }, [connect, fail, options.deps]);

  const cancel = useCallback(() => {
    activeRef.current = false;
    generationRef.current += 1;
    closeEvents();
    dispatch({ type: "reset", generation: generationRef.current });
  }, [closeEvents]);

  useEffect(() => () => {
    generationRef.current += 1;
    activeRef.current = false;
    closeEvents();
  }, [closeEvents]);

  return { state, active: isLoginActive(state), start, cancel } as const;
}

function parseLoginEvent(data: string): LoginEvent | null {
  try {
    const parsed = loginEventSchema.safeParse(JSON.parse(data) as unknown);
    return parsed.success ? parsed.data : null;
  } catch {
    return null;
  }
}

function toPublicError(error: unknown): PublicApiError {
  if (error instanceof PublicApiError) return error;
  return new PublicApiError({ status: 0, code: "LOGIN_FAILED", message: "登录失败，请重试。" });
}

function invalidEventError(): PublicApiError {
  return new PublicApiError({
    status: 0,
    code: "INVALID_SSE_EVENT",
    message: "登录状态无法识别，请重新开始。",
  });
}

function sseError(): PublicApiError {
  return new PublicApiError({
    status: 0,
    code: "SSE_CONNECTION_FAILED",
    message: "登录连接已中断，请重新开始。",
  });
}

function sessionClaimError(): PublicApiError {
  return new PublicApiError({
    status: 401,
    code: "SESSION_CLAIM_FAILED",
    message: "登录会话未能完成，请重新扫码。",
  });
}
