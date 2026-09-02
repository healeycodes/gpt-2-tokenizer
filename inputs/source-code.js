// Synthetic event reducer: comments, identifiers, strings, numbers, and nesting.
const DEFAULT_LIMIT = 12;

export function summarizeEvents(events, options = {}) {
  const { limit = DEFAULT_LIMIT, includeKinds = true } = options;
  const counts = new Map();
  const accepted = [];

  for (const event of events) {
    if (!event?.name || event.disabled === true) continue;
    const kind = event.kind ?? "unknown";
    counts.set(kind, (counts.get(kind) ?? 0) + 1);

    if (accepted.length < limit) {
      accepted.push({
        id: String(event.id ?? "local"),
        label: `${event.name.trim()} (${kind})`,
        at: new Date(event.at ?? 0).toISOString(),
      });
    }
  }

  return {
    total: accepted.length,
    truncated: events.length > accepted.length,
    kinds: includeKinds ? Object.fromEntries(counts) : undefined,
    entries: accepted,
  };
}

console.log(summarizeEvents([{ id: 7, name: "check-in", kind: "rail" }]));
