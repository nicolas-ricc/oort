import { MutableRefObject, useEffect, useRef, useState } from 'react';
import { SpaceshipState } from '@/cloud/spaceship/useSpaceshipControls';
import { Simulation } from '@/App';

type Props = {
  shipStateRef: MutableRefObject<SpaceshipState | null>;
  simulationData: Simulation;
  nearestPlanet: { concepts: string[]; distance: number } | null;
  regionName: string | null;
  nearbyPlanets: { concepts: string[]; distance: number }[];
  discoveredCount: number;
  totalCount: number;
};

const CONTROLS_HINT_KEY = 'oort-controls-hint-shown';

export function HUD({
  shipStateRef,
  nearestPlanet,
  regionName,
  nearbyPlanets,
  discoveredCount,
  totalCount,
}: Props) {
  const [speed, setSpeed] = useState(0);
  const [maxSpeed, setMaxSpeed] = useState(240);
  const [isBoosting, setIsBoosting] = useState(false);
  const [heading, setHeading] = useState(0);
  const [pitch, setPitch] = useState(0);
  const [showHint, setShowHint] = useState(() => !localStorage.getItem(CONTROLS_HINT_KEY));
  const rafRef = useRef<number>(0);

  // Update HUD values at ~10fps
  useEffect(() => {
    let frameCount = 0;
    const update = () => {
      frameCount++;
      if (frameCount % 6 === 0) {
        const state = shipStateRef.current;
        if (state) {
          setSpeed(state.speed);
          setMaxSpeed((state as any).maxSpeed ?? 240);
          setIsBoosting(state.isBoosting);

          // Extract yaw/pitch from quaternion
          const q = state.quaternion;
          const yaw = Math.atan2(
            2 * (q.w * q.y + q.x * q.z),
            1 - 2 * (q.y * q.y + q.x * q.x)
          );
          const pitchVal = Math.asin(
            Math.max(-1, Math.min(1, 2 * (q.w * q.x - q.z * q.y)))
          );
          setHeading(((yaw * 180 / Math.PI) + 360) % 360);
          setPitch(pitchVal * 180 / Math.PI);
        }
      }
      rafRef.current = requestAnimationFrame(update);
    };
    rafRef.current = requestAnimationFrame(update);
    return () => cancelAnimationFrame(rafRef.current);
  }, [shipStateRef]);

  // Auto-hide controls hint
  useEffect(() => {
    if (!showHint) return;
    const timer = setTimeout(() => {
      setShowHint(false);
      localStorage.setItem(CONTROLS_HINT_KEY, '1');
    }, 10000);
    return () => clearTimeout(timer);
  }, [showHint]);

  const speedPercent = Math.min(speed / Math.max(maxSpeed, 1), 1) * 100;

  return (
    <div className="fixed inset-0 pointer-events-none z-30 font-mono">
      {/* Region name — top left */}
      {regionName && (
        <div className="absolute top-4 left-4 bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-3 py-1.5">
          <div className="text-terminal-muted text-[10px] uppercase tracking-wider">Region</div>
          <div className="text-terminal-text text-sm uppercase">{regionName}</div>
        </div>
      )}

      {/* Compass — top center */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-4 py-1.5 text-center">
        <div className="text-terminal-text text-sm tabular-nums">
          {heading.toFixed(0).padStart(3, '0')}°
          <span className="text-terminal-muted text-xs ml-2">
            {pitch > 0 ? '+' : ''}{pitch.toFixed(0)}°
          </span>
        </div>
      </div>

      {/* Discovery count — top right */}
      <div className="absolute top-4 right-4 bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-3 py-1.5">
        <div className="text-terminal-muted text-[10px] uppercase tracking-wider">Discovered</div>
        <div className="text-terminal-text text-sm tabular-nums">{discoveredCount}/{totalCount}</div>
      </div>

      {/* Nearby concepts — left side */}
      {nearbyPlanets.length > 0 && (
        <div className="absolute top-24 left-4 bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-3 py-2 max-w-[200px]">
          <div className="text-terminal-muted text-[10px] uppercase tracking-wider mb-1">Nearby</div>
          {nearbyPlanets.slice(0, 5).map((planet, i) => (
            <div key={i} className="text-xs text-terminal-text truncate flex justify-between gap-2">
              <span className="truncate">{planet.concepts[0]}</span>
              <span className="text-terminal-muted shrink-0">{planet.distance.toFixed(1)}</span>
            </div>
          ))}
        </div>
      )}

      {/* Speed bar — bottom center */}
      <div className="absolute bottom-6 left-1/2 -translate-x-1/2 w-[300px]">
        <div className="bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-3 py-2">
          <div className="flex justify-between text-[10px] text-terminal-muted uppercase tracking-wider mb-1">
            <span>Speed</span>
            <span className={isBoosting ? 'text-blue-400' : ''}>{isBoosting ? 'BOOST' : ''}</span>
          </div>
          <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-75 rounded-full ${
                isBoosting ? 'bg-blue-400 shadow-[0_0_8px_rgba(96,165,250,0.5)]' : 'bg-terminal-text'
              }`}
              style={{ width: `${speedPercent}%` }}
            />
          </div>
          <div className="text-terminal-text text-xs text-center mt-1 tabular-nums">
            {speed.toFixed(1)}
          </div>
        </div>
      </div>

      {/* Controls hint — bottom right, fades after 10s */}
      {showHint && (
        <div className="absolute bottom-6 right-4 bg-zinc-900/40 backdrop-blur-sm border border-terminal-border/30 px-3 py-2 text-xs text-terminal-muted animate-fade-in">
          <div>WASD fly</div>
          <div>Mouse look</div>
          <div>Shift boost</div>
          <div>` terminal</div>
        </div>
      )}
    </div>
  );
}
