import * as PIXI from 'pixi.js';
import { CANVAS_WIDTH, CANVAS_HEIGHT, MIN_ZOOM, MAX_ZOOM, ZOOM_SPEED, TAP_THRESHOLD_PX, TAP_THRESHOLD_MS } from './constants';

export class CameraController {
  private world: PIXI.Container;
  private app: PIXI.Application;
  private zoom = 1;
  private dragging = false;
  private lastPointer = { x: 0, y: 0 };

  // Touch state
  private touches: Map<number, { x: number; y: number }> = new Map();
  private lastPinchDist = 0;

  // Tap detection state: distinguish taps from drags on touch devices
  private touchStartTime = 0;
  private touchStartPos = { x: 0, y: 0 };
  private touchMoved = false;

  constructor(world: PIXI.Container, app: PIXI.Application) {
    this.world = world;
    this.app = app;

    const view = app.view as HTMLCanvasElement;

    // Mouse wheel zoom
    view.addEventListener('wheel', this.onWheel, { passive: false });

    // Mouse drag pan
    view.addEventListener('mousedown', this.onMouseDown);
    view.addEventListener('mousemove', this.onMouseMove);
    view.addEventListener('mouseup', this.onMouseUp);
    view.addEventListener('mouseleave', this.onMouseUp);

    // Touch events
    view.addEventListener('touchstart', this.onTouchStart, { passive: false });
    view.addEventListener('touchmove', this.onTouchMove, { passive: false });
    view.addEventListener('touchend', this.onTouchEnd);
    view.addEventListener('touchcancel', this.onTouchEnd);
  }

  fitToBoard(): void {
    const viewWidth = this.app.screen.width;
    const viewHeight = this.app.screen.height;
    const scaleX = viewWidth / CANVAS_WIDTH;
    const scaleY = viewHeight / CANVAS_HEIGHT;
    this.zoom = Math.min(scaleX, scaleY, 1);
    this.world.scale.set(this.zoom);
    this.world.position.set(
      (viewWidth - CANVAS_WIDTH * this.zoom) / 2,
      (viewHeight - CANVAS_HEIGHT * this.zoom) / 2,
    );
  }

  screenToBoard(screenX: number, screenY: number): { x: number; y: number } {
    const worldX = (screenX - this.world.position.x) / this.world.scale.x;
    const worldY = (screenY - this.world.position.y) / this.world.scale.y;
    return { x: worldX, y: worldY };
  }

  boardToScreen(boardX: number, boardY: number): { x: number; y: number } {
    return {
      x: boardX * this.world.scale.x + this.world.position.x,
      y: boardY * this.world.scale.y + this.world.position.y,
    };
  }

  private onWheel = (e: WheelEvent): void => {
    e.preventDefault();

    const oldZoom = this.zoom;
    const delta = e.deltaY > 0 ? -ZOOM_SPEED : ZOOM_SPEED;
    this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, this.zoom + delta));

    // Zoom toward cursor
    const rect = (this.app.view as HTMLCanvasElement).getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    const ratio = this.zoom / oldZoom;
    this.world.position.x = mouseX - (mouseX - this.world.position.x) * ratio;
    this.world.position.y = mouseY - (mouseY - this.world.position.y) * ratio;
    this.world.scale.set(this.zoom);
  };

  private onMouseDown = (e: MouseEvent): void => {
    if (e.button === 1 || e.button === 2) {
      // Middle or right click to pan
      this.dragging = true;
      this.lastPointer = { x: e.clientX, y: e.clientY };
    }
  };

  private onMouseMove = (e: MouseEvent): void => {
    if (!this.dragging) return;
    const dx = e.clientX - this.lastPointer.x;
    const dy = e.clientY - this.lastPointer.y;
    this.world.position.x += dx;
    this.world.position.y += dy;
    this.lastPointer = { x: e.clientX, y: e.clientY };
  };

  private onMouseUp = (): void => {
    this.dragging = false;
  };

  private onTouchStart = (e: TouchEvent): void => {
    e.preventDefault();
    for (let i = 0; i < e.changedTouches.length; i++) {
      const t = e.changedTouches[i];
      this.touches.set(t.identifier, { x: t.clientX, y: t.clientY });
    }

    // Track single-finger tap detection
    if (this.touches.size === 1) {
      const t = e.changedTouches[0];
      this.touchStartTime = Date.now();
      this.touchStartPos = { x: t.clientX, y: t.clientY };
      this.touchMoved = false;
    }

    if (this.touches.size === 2) {
      this.lastPinchDist = this.getPinchDist();
    }
  };

  private onTouchMove = (e: TouchEvent): void => {
    e.preventDefault();

    if (this.touches.size === 1) {
      const t = e.changedTouches[0];
      const prev = this.touches.get(t.identifier);
      if (prev) {
        // Check if movement exceeds tap threshold
        const dx = t.clientX - this.touchStartPos.x;
        const dy = t.clientY - this.touchStartPos.y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist > TAP_THRESHOLD_PX) {
          this.touchMoved = true;
        }

        // Only pan if we've exceeded the tap threshold (confirmed drag)
        if (this.touchMoved) {
          const panDx = t.clientX - prev.x;
          const panDy = t.clientY - prev.y;
          this.world.position.x += panDx;
          this.world.position.y += panDy;
        }

        this.touches.set(t.identifier, { x: t.clientX, y: t.clientY });
      }
    } else if (this.touches.size === 2) {
      // Pinch zoom
      this.touchMoved = true; // Pinch is never a tap
      for (let i = 0; i < e.changedTouches.length; i++) {
        const t = e.changedTouches[i];
        this.touches.set(t.identifier, { x: t.clientX, y: t.clientY });
      }

      const newDist = this.getPinchDist();
      if (this.lastPinchDist > 0) {
        const scale = newDist / this.lastPinchDist;
        const oldZoom = this.zoom;
        this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, this.zoom * scale));

        // Zoom toward pinch center
        const center = this.getPinchCenter();
        const ratio = this.zoom / oldZoom;
        this.world.position.x = center.x - (center.x - this.world.position.x) * ratio;
        this.world.position.y = center.y - (center.y - this.world.position.y) * ratio;
        this.world.scale.set(this.zoom);
      }
      this.lastPinchDist = newDist;
    }
  };

  private onTouchEnd = (e: TouchEvent): void => {
    // Detect single-finger taps and dispatch to PixiJS interaction system
    if (this.touches.size === 1 && !this.touchMoved) {
      const elapsed = Date.now() - this.touchStartTime;
      if (elapsed < TAP_THRESHOLD_MS) {
        // This is a tap — dispatch a synthetic pointer event to PixiJS
        const t = e.changedTouches[0];
        const view = this.app.view as HTMLCanvasElement;
        const pointerEvent = new PointerEvent('pointerdown', {
          clientX: t.clientX,
          clientY: t.clientY,
          bubbles: true,
          pointerId: t.identifier,
          pointerType: 'touch',
        });
        view.dispatchEvent(pointerEvent);
      }
    }

    for (let i = 0; i < e.changedTouches.length; i++) {
      this.touches.delete(e.changedTouches[i].identifier);
    }
    if (this.touches.size < 2) {
      this.lastPinchDist = 0;
    }
  };

  private getPinchDist(): number {
    const pts = [...this.touches.values()];
    if (pts.length < 2) return 0;
    const dx = pts[1].x - pts[0].x;
    const dy = pts[1].y - pts[0].y;
    return Math.sqrt(dx * dx + dy * dy);
  }

  private getPinchCenter(): { x: number; y: number } {
    const pts = [...this.touches.values()];
    if (pts.length < 2) return { x: 0, y: 0 };
    return {
      x: (pts[0].x + pts[1].x) / 2,
      y: (pts[0].y + pts[1].y) / 2,
    };
  }

  destroy(): void {
    const view = this.app.view as HTMLCanvasElement;
    view.removeEventListener('wheel', this.onWheel);
    view.removeEventListener('mousedown', this.onMouseDown);
    view.removeEventListener('mousemove', this.onMouseMove);
    view.removeEventListener('mouseup', this.onMouseUp);
    view.removeEventListener('mouseleave', this.onMouseUp);
    view.removeEventListener('touchstart', this.onTouchStart);
    view.removeEventListener('touchmove', this.onTouchMove);
    view.removeEventListener('touchend', this.onTouchEnd);
    view.removeEventListener('touchcancel', this.onTouchEnd);
  }
}
