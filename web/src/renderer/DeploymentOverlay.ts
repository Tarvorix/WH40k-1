import * as PIXI from 'pixi.js';
import type { BoardView, DeploymentZoneView } from '@/types/game';
import { PX_PER_INCH, DEPLOYMENT_ZONE_ALPHA, DEPLOYMENT_ZONE_BORDER_ALPHA, DEPLOYMENT_ZONE_BORDER_WIDTH } from './constants';
import { getFactionPrimaryColor } from '@/utils/colors';

export class DeploymentOverlay {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(board: BoardView, phase?: string, decisionOwner?: number): void {
    this.container.removeChildren();

    const isDeployment = phase === 'PreBattle';

    // Combat Patrol deployment zones
    for (const zone of board.deployment_zones) {
      const isActiveZone = isDeployment && zone.player === decisionOwner;
      this.drawZone(zone, isActiveZone);
    }

    // Boarding Actions entry zones
    if (board.entry_zones) {
      for (const ez of board.entry_zones) {
        const isActiveZone = isDeployment && ez.player === decisionOwner;
        this.drawEntryZone(ez, isActiveZone);
      }
    }
  }

  private drawEntryZone(
    zone: { vertices: { x: number; y: number }[]; player: number | null; name: string; role: string },
    isActive: boolean,
  ): void {
    if (zone.vertices.length < 3) return;

    const g = new PIXI.Graphics();
    const color = zone.player != null ? getFactionPrimaryColor(zone.player) : 0x888888;

    const fillAlpha = isActive ? 0.25 : DEPLOYMENT_ZONE_ALPHA;
    const borderAlpha = isActive ? 0.7 : DEPLOYMENT_ZONE_BORDER_ALPHA;
    const borderWidth = isActive ? 3 : DEPLOYMENT_ZONE_BORDER_WIDTH;

    g.beginFill(color, fillAlpha);
    const first = zone.vertices[0];
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();
    g.endFill();

    g.lineStyle(borderWidth, color, borderAlpha);
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();

    // Label
    const cx = zone.vertices.reduce((s, v) => s + v.x, 0) / zone.vertices.length * PX_PER_INCH;
    const cy = zone.vertices.reduce((s, v) => s + v.y, 0) / zone.vertices.length * PX_PER_INCH;
    const label = new PIXI.Text(zone.name, {
      fontSize: 9,
      fill: 0xffffff,
      fontWeight: 'bold',
    });
    label.anchor.set(0.5, 0.5);
    label.position.set(cx, cy);
    g.addChild(label);

    if (isActive) {
      g.eventMode = 'static';
      g.cursor = 'crosshair';
    }

    this.container.addChild(g);
  }

  private drawZone(zone: DeploymentZoneView, isActive: boolean): void {
    if (zone.vertices.length < 3) return;

    const g = new PIXI.Graphics();
    const color = getFactionPrimaryColor(zone.player);

    // Use brighter alpha for active deployment zone
    const fillAlpha = isActive ? 0.25 : DEPLOYMENT_ZONE_ALPHA;
    const borderAlpha = isActive ? 0.7 : DEPLOYMENT_ZONE_BORDER_ALPHA;
    const borderWidth = isActive ? 3 : DEPLOYMENT_ZONE_BORDER_WIDTH;

    // Fill polygon
    g.beginFill(color, fillAlpha);
    const first = zone.vertices[0];
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();
    g.endFill();

    // Border
    g.lineStyle(borderWidth, color, borderAlpha);
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();

    // Make clickable during active deployment
    if (isActive) {
      g.eventMode = 'static';
      g.cursor = 'crosshair';
    }

    this.container.addChild(g);
  }
}
