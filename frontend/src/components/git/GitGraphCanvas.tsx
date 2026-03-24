import { useRef, useEffect } from 'react';
import { getRailColor, type GraphConnection } from '../../lib/graphLayout';

interface GitGraphCanvasProps {
  connections: GraphConnection[];
  railCount: number;
  rowHeight: number;
  railWidth: number;
  nodeRadius: number;
  width: number;
  height: number;
}

export function GitGraphCanvas({
  connections,
  railCount,
  rowHeight,
  railWidth,
  nodeRadius,
  width,
  height,
}: GitGraphCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Handle device pixel ratio for sharp rendering
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Set line properties
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    // Draw connections
    for (const conn of connections) {
      const fromX = (conn.fromRail + 0.5) * railWidth;
      const fromY = conn.fromRow * rowHeight + rowHeight / 2;
      const toX = (conn.toRail + 0.5) * railWidth;
      const toY = conn.toRow * rowHeight + rowHeight / 2;

      // Use the color of the rail we're drawing from (child commit)
      ctx.strokeStyle = getRailColor(conn.fromRail);

      ctx.beginPath();

      if (conn.type === 'straight') {
        // Simple vertical line
        ctx.moveTo(fromX, fromY + nodeRadius);
        ctx.lineTo(toX, toY - nodeRadius);
      } else if (conn.type === 'merge-left' || conn.type === 'merge-right') {
        // Curved connection for merges
        // Start from the node
        ctx.moveTo(fromX, fromY + nodeRadius);

        // Calculate control points for a smooth bezier curve
        const midY = (fromY + toY) / 2;

        if (conn.fromRow + 1 === conn.toRow) {
          // Adjacent rows - use a simple curve
          ctx.bezierCurveTo(
            fromX, midY,          // Control point 1: straight down from start
            toX, midY,            // Control point 2: straight up from end
            toX, toY - nodeRadius // End point
          );
        } else {
          // Non-adjacent rows - draw vertical line first, then curve
          const curveStartY = toY - rowHeight;

          // Vertical line down
          ctx.lineTo(fromX, curveStartY);

          // Curve to the target rail
          ctx.bezierCurveTo(
            fromX, curveStartY + rowHeight * 0.5,  // Control point 1
            toX, curveStartY + rowHeight * 0.5,    // Control point 2
            toX, toY - nodeRadius                   // End point
          );
        }
      }

      ctx.stroke();
    }
  }, [connections, railCount, rowHeight, railWidth, nodeRadius, width, height]);

  return (
    <canvas
      ref={canvasRef}
      className="absolute top-0 left-0"
      style={{
        width,
        height,
        pointerEvents: 'none',
      }}
    />
  );
}
