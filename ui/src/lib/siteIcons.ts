import type { SiteIcon } from "../bindings/SiteIcon";

/** The key a site's icon is stored under: its host name in lower case. Null for no address. */
export function siteIconKey(url: string): string | null {
  try {
    const host = new URL(url).hostname.toLowerCase();
    return host === "" ? null : host;
  } catch {
    return null;
  }
}

/**
 * The icons by host, as addresses the webview can load (`toSrc` turns a file path into one). The
 * version makes a refreshed icon replace the one the webview cached.
 */
export function siteIconUrls(
  icons: readonly SiteIcon[],
  toSrc: (path: string) => string,
): Record<string, string> {
  const urls: Record<string, string> = {};
  for (const icon of icons) urls[icon.host] = `${toSrc(icon.path)}?v=${icon.version}`;
  return urls;
}

/** The icon address for a site, or null when there is none (yet). */
export function siteIconUrl(
  urls: Readonly<Record<string, string>>,
  siteUrl: string,
): string | null {
  const key = siteIconKey(siteUrl);
  return key === null ? null : (urls[key] ?? null);
}
