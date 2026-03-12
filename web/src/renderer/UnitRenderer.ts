import * as PIXI from 'pixi.js';
import type { UnitView, PlayerView } from '@/types/game';
import {
  PX_PER_INCH,
  UNIT_CIRCLE_RADIUS,
  UNIT_SELECTED_RING_RADIUS,
  UNIT_TARGET_RING_RADIUS,
  UNIT_LABEL_FONT_SIZE,
  WOUND_BAR_WIDTH,
  WOUND_BAR_HEIGHT,
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

    const x = unit.position.x * PX_PER_INCH;
    const y = unit.position.y * PX_PER_INCH;
    const primaryColor = getFactionPrimaryColor(factionId);
    const darkColor = getFactionDarkColor(factionId);

    const unitContainer = new PIXI.Container();
    unitContainer.position.set(x, y);

    // Selection ring
    if (isSelected) {
      const ring = new PIXI.Graphics();
      ring.lineStyle(2, 0x00FF00, 0.8);
      ring.drawCircle(0, 0, UNIT_SELECTED_RING_RADIUS);
      unitContainer.addChild(ring);
    }

    // Target ring
    if (isTarget) {
      const ring = new PIXI.Graphics();
      ring.lineStyle(2, 0xFF0000, 0.8);
      ring.drawCircle(0, 0, UNIT_TARGET_RING_RADIUS);
      unitContainer.addChild(ring);
    }

    // Hover highlight
    if (isHovered && !isSelected && !isTarget) {
      const ring = new PIXI.Graphics();
      ring.lineStyle(1, 0xFFFFFF, 0.5);
      ring.drawCircle(0, 0, UNIT_CIRCLE_RADIUS + 2);
      unitContainer.addChild(ring);
    }

    // Main circle
    const circle = new PIXI.Graphics();
    circle.beginFill(primaryColor, 0.9);
    circle.lineStyle(1, darkColor, 1);
    circle.drawCircle(0, 0, UNIT_CIRCLE_RADIUS);
    circle.endFill();
    unitContainer.addChild(circle);

    // Unit type icon (first letter of unit type keyword)
    const typeChar = unit.is_character ? 'C' : unit.is_infantry ? 'I' : 'U';
    const icon = new PIXI.Text(typeChar, {
      fontSize: 10,
      fill: 0x000000,
      fontWeight: 'bold',
      fontFamily: 'Inter',
    });
    icon.anchor.set(0.5);
    unitContainer.addChild(icon);

    // Model count badge (top-right)
    if (unit.models_alive > 1) {
      const badge = new PIXI.Graphics();
      badge.beginFill(darkColor, 0.9);
      badge.drawCircle(UNIT_CIRCLE_RADIUS - 2, -UNIT_CIRCLE_RADIUS + 2, 6);
      badge.endFill();

      const countText = new PIXI.Text(`${unit.models_alive}`, {
        fontSize: 7,
        fill: 0xFFFFFF,
        fontWeight: 'bold',
        fontFamily: 'Inter',
      });
      countText.anchor.set(0.5);
      countText.position.set(UNIT_CIRCLE_RADIUS - 2, -UNIT_CIRCLE_RADIUS + 2);

      unitContainer.addChild(badge);
      unitContainer.addChild(countText);
    }

    // Wound bar (below unit)
    const totalWoundsMax = unit.models.reduce((sum, m) => sum + m.wounds_max, 0);
    if (totalWoundsMax > 0) {
      const ratio = unit.total_wounds_remaining / totalWoundsMax;
      const barColor =
        ratio > 0.5 ? STATUS_COLORS.healthy : ratio > 0.25 ? STATUS_COLORS.wounded : STATUS_COLORS.critical;

      const bar = new PIXI.Graphics();
      // Background
      bar.beginFill(0x333333, 0.8);
      bar.drawRect(-WOUND_BAR_WIDTH / 2, UNIT_CIRCLE_RADIUS + 2, WOUND_BAR_WIDTH, WOUND_BAR_HEIGHT);
      bar.endFill();
      // Fill
      bar.beginFill(barColor, 0.9);
      bar.drawRect(
        -WOUND_BAR_WIDTH / 2,
        UNIT_CIRCLE_RADIUS + 2,
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
    nameLabel.position.set(0, UNIT_CIRCLE_RADIUS + WOUND_BAR_HEIGHT + 4);
    nameLabel.alpha = 0.7;
    unitContainer.addChild(nameLabel);

    // Battle-shocked indicator
    if (unit.battle_shocked) {
      const shockIndicator = new PIXI.Graphics();
      shockIndicator.beginFill(STATUS_COLORS.battleShocked, 0.6);
      shockIndicator.drawCircle(-UNIT_CIRCLE_RADIUS + 2, -UNIT_CIRCLE_RADIUS + 2, 4);
      shockIndicator.endFill();
      unitContainer.addChild(shockIndicator);
    }

    this.container.addChild(unitContainer);
  }
}
