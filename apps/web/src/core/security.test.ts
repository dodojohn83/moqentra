import { describe, expect, it } from "vitest";
import { isAllowedDownloadType, sanitizeTrustedHtml, stripSecrets } from "./security";

describe("security", () => {
  it("allows whitelisted download types", () => {
    expect(isAllowedDownloadType("image/png; charset=utf-8")).toBe(true);
    expect(isAllowedDownloadType("application/octet-stream")).toBe(false);
  });

  it("strips tokens and api keys from strings", () => {
    const input = "https://x.com?token=secret&api_key=xyz";
    expect(stripSecrets(input)).toBe("https://x.com?token=<redacted>&api_key=<redacted>");
  });

  it("strips quoted secrets with spaces", () => {
    const input = 'password="my secret" token=\'another key\'';
    expect(stripSecrets(input)).toBe("password=<redacted> token=<redacted>");
  });

  it("escapes html characters", () => {
    expect(sanitizeTrustedHtml("<script>alert('x')</script>")).toBe(
      "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;",
    );
  });
});
