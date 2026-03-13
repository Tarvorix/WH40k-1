import * as PIXI from 'pixi.js';
import type { UnitView, PlayerView, ModelView } from '@/types/game';
import {
  PX_PER_INCH,
  UNIT_CIRCLE_RADIUS,
  UNIT_LABEL_FONT_SIZE,
  WOUND_BAR_WIDTH,
  WOUND_BAR_HEIGHT,
  baseRadiusPx,
} from './constants';
import { getFactionPrimaryColor, getFactionDarkColor, STATUS_COLORS } from '@/utils/colors';

export class UnitRenderer {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(
    units: UnitView[],
    players: [PlayerView, PlayerView],
    selectedUnitId: number | null,
    targetUnitId: number | null,
    hoveredUnitId: number | null,
  ): void {
    this.container.removeChildren();

    for (const unit of units) {
      if (!unit.position || unit.status === 'Destroyed' || unit.status === 'Undeployed') continue;

      const factionId = players[unit.owner]?.faction_id ?? unit.owner;
      this.drawUnit(
        unit,
        factionId,
        unit.id === selectedUnitId,
        unit.id === targetUnitId,
        unit.id === hoveredUnitId,
      );
    }
  }

  private drawUnit(
    unit: UnitView,
    factionId: number,
    isSelected: boolean,
    isTarget: boolean,
    isHovered: boolean,
  ): void {
    if (!unit.position) return;

    const primaryColor = getFactionPrimaryColor(factionId);
    const darkColor = getFactionDarkColor(factionId);

    const unitContainer = new PIXI.Container();

    // Compute centroid from alive models for positioning labels/bars
    const aliveModels = unit.models.filter(m => m.alive);
    let centroidX: number;
    let centroidY: number;

    if (aliveModels.length > 0) {
      centroidX = aliveModels.reduce((sum, m) => sum + m.position.x, 0) / aliveModels.length * PX_PER_INCH;
      centroidY = aliveModels.reduce((sum, m) => sum + m.position.y, 0) / aliveModels.length * PX_PER_INCH;
    } else {
      centroidX = unit.position.x * PX_PER_INCH;
      centroidY = unit.position.y * PX_PER_INCH;
    }

    // Draw each alive model as an individual circle at its position
    for (const model of unit.models) {
      if (!model.alive) continue;

      const mx = model.position.x * PX_PER_INCH;
      const my = model.position.y * PX_PER_INCH;
      const radius = baseRadiusPx(model.base_size_mm);

      // Selection ring (green) on each model of selected unit
      if (isSelected) {
        const ring = new PIXI.Graphics();
        ring.lineStyle(2, 0x00FF00, 0.8);
        ring.drawCircle(mx, my, radius + 4);
        unitContainer.addChild(ring);
      }

      // Target ring (red) on each model of targeted unit
      if (isTarget) {
        const ring = new PIXI.Graphics();
        ring.lineStyle(2, 0xFF0000, 0.8);
        ring.drawCircle(mx, my, radius + 4);
        unitContainer.addChild(ring);
      }

      // Hover highlight
      if (isHovered && !isSelected && !isTarget) {
        const ring = new PIXI.Graphics();
        ring.lineStyle(1, 0xFFFFFF, 0.5);
        ring.drawCircle(mx, my, radius + 2);
        unitContainer.addChild(ring);
      }

      // Main model circle — true-to-scale base size
      const circle = new PIXI.Graphics();
      circle.beginFill(primaryColor, 0.9);
      // Leader models get a thicker, brighter border
      if (model.is_leader) {
        circle.lineStyle(2, 0xFFD700, 1);
      } else {
        circle.lineStyle(1, darkColor, 1);
      }
      circle.drawCircle(mx, my, radius);
      circle.endFill();
      unitContainer.addChild(circle);
    }

    // Unit type icon at centroid (only for single-model or compact display)
    const typeChar = unit.is_character ? 'C' : unit.is_infantry ? 'I' : 'U';
    const icon = new PIXI.Text(typeChar, {
      fontSize: 10,
      fill: 0x000000,
      fontWeight: 'bold',
      fontFamily: 'Inter',
    });
    icon.anchor.set(0.5);
    icon.position.set(centroidX, centroidY);
    unitContainer.addChild(icon);

    // Model count badge (top-right of centroid) for zoom-out readability
    // Use the largest base radius among alive models for badge offset
    const maxRadius = aliveModels.length > 0
      ? Math.max(...aliveModels.map(m => baseRadiusPx(m.base_size_mm)))
      : UNIT_CIRCLE_RADIUS;

    if (unit.models_alive > 1) {
      const badge = new PIXI.Graphics();
      badge.beginFill(darkColor, 0.9);
      badge.drawCircle(centroidX + maxRadius - 2, centroidY - maxRadius + 2, 6);
      badge.endFill();

      const countText = new PIXI.Text(`${unit.models_alive}`, {
        fontSize: 7,
        fill: 0xFFFFFF,
        fontWeight: 'bold',
        fontFamily: 'Inter',
      });
      countText.anchor.set(0.5);
      countText.position.set(centroidX + maxRadius - 2, centroidY - maxRadius + 2);

      unitContainer.addChild(badge);
      unitContainer.addChild(countText);
    }

    // Wound bar (below centroid)
    const totalWoundsMax = unit.models.reduce((sum, m) => sum + m.wounds_max, 0);
    if (totalWoundsMax > 0) {
      const ratio = unit.total_wounds_remaining / totalWoundsMax;
      const barColor =
        ratio > 0.5 ? STATUS_COLORS.healthy : ratio > 0.25 ? STATUS_COLORS.wounded : STATUS_COLORS.critical;

      const bar = new PIXI.Graphics();
      // Background
      bar.beginFill(0x333333, 0.8);
      bar.drawRect(centroidX - WOUND_BAR_WIDTH / 2, centroidY + maxRadius + 2, WOUND_BAR_WIDTH, WOUND_BAR_HEIGHT);
      bar.endFill();
      // Fill
      bar.beginFill(barColor, 0.9);
      bar.drawRect(
        centroidX - WOUND_BAR_WIDTH / 2,
        centroidY + maxRadius + 2,
        WOUND_BAR_WIDTH * ratio,
        WOUND_BAR_HEIGHT,
      );
      bar.endFill();

      unitContainer.addChild(bar);
    }

    // Unit name label (below wound bar)
    const nameLabel = new PIXI.Text(unit.name, {
      fontSize: UNIT_LABEL_FONT_SIZE,
      fill: 0xCCCCCC,
      fontFamily: 'Inter',
    });
    nameLabel.anchor.set(0.5, 0);
    nameLabel.position.set(centroidX, centroidY + maxRadius + WOUND_BAR_HEIGHT + 4);
    nameLabel.alpha = 0.7;
    unitContainer.addChild(nameLabel);

    // Battle-shocked indicator (at centroid)
    if (unit.battle_shocked) {
      const shockIndicator = new PIXI.Graphics();
      shockIndicator.beginFill(STATUS_COLORS.battleShocked, 0.6);
      shockIndicator.drawCircle(centroidX - maxRadius + 2, centroidY - maxRadius + 2, 4);
      shockIndicator.endFill();
      unitContainer.addChild(shockIndicator);
    }

    this.container.addChild(unitContainer);
  }
}
