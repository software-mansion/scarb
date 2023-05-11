export function platform(): "windows" | "macos" | "linux" | "unknown" {
  const platform = window.navigator?.platform;
  if (platform.startsWith("Win")) {
    return "windows";
  } else if (platform.startsWith("Mac")) {
    return "macos";
  } else if (platform.startsWith("Linux")) {
    return "linux";
  } else {
    return "unknown";
  }
}
