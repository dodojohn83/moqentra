/**
 * Support tier labels for the R2 web console.
 *
 * These labels intentionally avoid a single green "available" wording so that
 * preview, compile-only and blocked tiers are not confused with fully supported
 * production targets.
 */

export type SupportTier = "verified" | "preview" | "compile-only" | "blocked" | "unsupported";

export interface TierPresentation {
  label: string;
  color: "green" | "blue" | "amber" | "red" | "gray";
  description: string;
}

const TIER_META: Record<SupportTier, TierPresentation> = {
  verified: {
    label: "Verified",
    color: "green",
    description: "Tested and approved for production workloads.",
  },
  preview: {
    label: "Preview",
    color: "blue",
    description: "Functional but not yet fully certified for production.",
  },
  "compile-only": {
    label: "Compile Only",
    color: "amber",
    description: "Conversion succeeds but runtime validation is incomplete.",
  },
  blocked: {
    label: "Blocked",
    color: "red",
    description: "This target cannot be scheduled with the selected configuration.",
  },
  unsupported: {
    label: "Unsupported",
    color: "gray",
    description: "No converter or runtime support is available.",
  },
};

export function tierPresentation(tier: SupportTier): TierPresentation {
  return TIER_META[tier] ?? TIER_META.unsupported;
}

export function tierFromSupportTierLevel(level: string): SupportTier {
  switch (level.toLowerCase()) {
    case "verified":
      return "verified";
    case "preview":
      return "preview";
    case "compile-only":
    case "compileonly":
      return "compile-only";
    case "blocked":
      return "blocked";
    default:
      return "unsupported";
  }
}
