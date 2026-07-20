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
  return input
    .replace(/token=[^&\s]+/gi, "token=<redacted>")
    .replace(/api[_-]?key=[^&\s]+/gi, "api_key=<redacted>");
}

export function sanitizeTrustedHtml(dirty: string): string {
  return dirty
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
