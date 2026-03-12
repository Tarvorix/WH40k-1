import { LONG_PRESS_MS } from './constants';

interface TouchHandlerCallbacks {
  onLongPress: (x: number, y: number) => void;
  onTap: (x: number, y: number) => void;
}

export class TouchHandler {
  private longPressTimer: ReturnType<typeof setTimeout> | null = null;
  private startPos = { x: 0, y: 0 };
  private element: HTMLElement;
  private callbacks: TouchHandlerCallbacks;

  constructor(element: HTMLElement, callbacks: TouchHandlerCallbacks) {
    this.element = element;
    this.callbacks = callbacks;

    element.addEventListener('touchstart', this.onTouchStart, { passive: false });
    element.addEventListener('touchmove', this.onTouchMove, { passive: false });
    element.addEventListener('touchend', this.onTouchEnd);
    element.addEventListener('touchcancel', this.onTouchCancel);
  }

  private onTouchStart = (e: TouchEvent): void => {
    if (e.touches.length !== 1) {
      this.cancelLongPress();
      return;
    }

    const touch = e.touches[0];
    this.startPos = { x: touch.clientX, y: touch.clientY };

    this.longPressTimer = setTimeout(() => {
      this.callbacks.onLongPress(this.startPos.x, this.startPos.y);
      this.longPressTimer = null;
    }, LONG_PRESS_MS);
  };

  private onTouchMove = (e: TouchEvent): void => {
    if (!this.longPressTimer) return;

    const touch = e.touches[0];
    const dx = touch.clientX - this.startPos.x;
    const dy = touch.clientY - this.startPos.y;
    const dist = Math.sqrt(dx * dx + dy * dy);

    // Cancel long press if finger moves too far
    if (dist > 10) {
      this.cancelLongPress();
    }
  };

  private onTouchEnd = (e: TouchEvent): void => {
    if (this.longPressTimer) {
      // Was a tap, not a long press
      this.cancelLongPress();
      if (e.changedTouches.length === 1) {
        const touch = e.changedTouches[0];
        this.callbacks.onTap(touch.clientX, touch.clientY);
      }
    }
  };

  private onTouchCancel = (): void => {
    this.cancelLongPress();
  };

  private cancelLongPress(): void {
    if (this.longPressTimer) {
      clearTimeout(this.longPressTimer);
      this.longPressTimer = null;
    }
  }

  destroy(): void {
    this.cancelLongPress();
    this.element.removeEventListener('touchstart', this.onTouchStart);
    this.element.removeEventListener('touchmove', this.onTouchMove);
    this.element.removeEventListener('touchend', this.onTouchEnd);
    this.element.removeEventListener('touchcancel', this.onTouchCancel);
  }
}
