export const ALLOWED_DOWNLOAD_TYPES = new Set([
  "image/png",
  "image/jpeg",
  "image/webp",
  "text/plain",
  "application/json",
  "application/zip",
]);

export function isAllowedDownloadType(contentType: string): boolean {
  const normalized = contentType.split(";")[0].trim().toLowerCase();
  return ALLOWED_DOWNLOAD_TYPES.has(normalized);
}

export function stripSecrets(input: string): string {
  // Values may be quoted (single or double) or unterminated; unquoted values
  // stop at whitespace or an ampersand to stay query-string friendly.
  const value = '(?:"[^"]*"|\'[^\']*\'|[^&\\s]+)';
  const patterns = [
    new RegExp(`password=${value}`, "gi"),
    new RegExp(`token=${value}`, "gi"),
    new RegExp(`api[_-]?key=${value}`, "gi"),
    new RegExp(`private[_-]?key=${value}`, "gi"),
    new RegExp(`secret=${value}`, "gi"),
  ];
  return patterns.reduce(
    (acc, pattern) =>
      acc.replace(pattern, (match) => {
        const key = match.slice(0, match.indexOf("=") ?? 0);
        return `${key}=<redacted>`;
      }),
    input,
  );
}

export function sanitizeTrustedHtml(dirty: string): string {
  return dirty
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
