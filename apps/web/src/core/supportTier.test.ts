import { describe, expect, it } from "vitest";
import { tierFromSupportTierLevel, tierPresentation } from "./supportTier";

describe("support tier presentation", () => {
  it("verified is green and production ready", () => {
    const t = tierPresentation("verified");
    expect(t.label).toBe("Verified");
    expect(t.color).toBe("green");
  });

  it("compile only is amber, not green", () => {
    const t = tierPresentation("compile-only");
    expect(t.label).toBe("Compile Only");
    expect(t.color).toBe("amber");
    expect(t.color).not.toBe("green");
  });

  it("blocked is red", () => {
    const t = tierPresentation("blocked");
    expect(t.color).toBe("red");
  });

  it("parse ignores case and normalizes aliases", () => {
    expect(tierFromSupportTierLevel("CompileOnly")).toBe("compile-only");
    expect(tierFromSupportTierLevel("PREVIEW")).toBe("preview");
    expect(tierFromSupportTierLevel("unknown")).toBe("unsupported");
  });
});
