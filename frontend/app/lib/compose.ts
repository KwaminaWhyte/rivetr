/**
 * Names of the services in a compose file (the keys under top-level
 * `services:`). A light line-based parse: enough for pickers, not validation.
 */
export function composeServiceNames(content: string): string[] {
  const names: string[] = [];
  let inServices = false;
  let childIndent = -1;
  for (const raw of content.split("\n")) {
    const line = raw.replace(/\s+#.*$/, "");
    if (!line.trim() || line.trim().startsWith("#")) continue;
    const indent = line.search(/\S/);
    if (indent === 0) {
      inServices = /^services:\s*$/.test(line);
      childIndent = -1;
      continue;
    }
    if (!inServices) continue;
    if (childIndent === -1) childIndent = indent;
    if (indent === childIndent) {
      const m = line.trim().match(/^["']?([A-Za-z0-9._-]+)["']?:/);
      if (m) names.push(m[1]);
    }
  }
  return names;
}
