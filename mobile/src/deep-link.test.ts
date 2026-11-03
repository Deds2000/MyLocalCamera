import { afterEach, describe, expect, it, vi } from "vitest";
import { logDeepLink } from "./deep-link";

describe("logDeepLink", () => {
  afterEach(() => vi.restoreAllMocks());

  it("logs routing metadata without the pairing token or full URI", () => {
    const token = "0123456789abcdef0123456789abcdef";
    const uri = `localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=${token}`;
    const info = vi.spyOn(console, "info").mockImplementation(() => {});

    logDeepLink(uri);

    expect(info).toHaveBeenCalledOnce();
    expect(info.mock.calls[0]?.[0]).toBe(
      "[deep-link] urlOpen scheme=localcamera host=connect port=48650",
    );
    expect(info.mock.calls[0]?.[0]).not.toContain(token);
    expect(info.mock.calls[0]?.[0]).not.toContain("token=");
  });

  it("does not log malformed URI details", () => {
    const info = vi.spyOn(console, "info").mockImplementation(() => {});

    logDeepLink("not-a-uri-with-secret-fragment");

    expect(info).toHaveBeenCalledWith("[deep-link] urlOpen (uri invalida, sem detalhes)");
  });
});
