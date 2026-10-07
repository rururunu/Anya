export interface PetMonitorBounds {
  position: { x: number; y: number };
  size: { width: number; height: number };
}

export function clampPetPosition(
  position: { x: number; y: number },
  windowSize: { width: number; height: number },
  monitors: readonly PetMonitorBounds[],
): { x: number; y: number } {
  if (!monitors.length) return position;
  const centerX = position.x + windowSize.width / 2;
  const centerY = position.y + windowSize.height / 2;
  const monitor = monitors.reduce((nearest, candidate) => {
    const distance = (bounds: PetMonitorBounds) => {
      const x = Math.max(
        bounds.position.x,
        Math.min(centerX, bounds.position.x + bounds.size.width),
      );
      const y = Math.max(
        bounds.position.y,
        Math.min(centerY, bounds.position.y + bounds.size.height),
      );
      return (centerX - x) ** 2 + (centerY - y) ** 2;
    };
    return distance(candidate) < distance(nearest) ? candidate : nearest;
  });
  return {
    x: Math.round(
      Math.max(
        monitor.position.x,
        Math.min(position.x, monitor.position.x + monitor.size.width - windowSize.width),
      ),
    ),
    y: Math.round(
      Math.max(
        monitor.position.y,
        Math.min(position.y, monitor.position.y + monitor.size.height - windowSize.height),
      ),
    ),
  };
}

export function getPetEdgePressure(
  requested: { x: number; y: number },
  bounded: { x: number; y: number },
): { x: number; y: number } {
  return {
    x: Math.max(-1, Math.min(1, (requested.x - bounded.x) / 65)),
    y: Math.max(-1, Math.min(1, (requested.y - bounded.y) / 65)),
  };
}
