import * as PIXI from 'pixi.js';

// Procedurally generate unit type icons
export class SpriteFactory {
  static createInfantryIcon(color: number, size: number = 12): PIXI.Graphics {
    const g = new PIXI.Graphics();
    g.beginFill(color);
    // Simple infantry silhouette: cross shape
    const s = size / 4;
    g.drawRect(-s, -size / 2, s * 2, size); // vertical
    g.drawRect(-size / 2, -s, size, s * 2); // horizontal
    g.endFill();
    return g;
  }

  static createCharacterIcon(color: number, size: number = 12): PIXI.Graphics {
    const g = new PIXI.Graphics();
    g.beginFill(color);
    // Star shape for characters
    const outer = size / 2;
    const inner = size / 4;
    const points: number[] = [];
    for (let i = 0; i < 5; i++) {
      const outerAngle = (Math.PI * 2 * i) / 5 - Math.PI / 2;
      const innerAngle = outerAngle + Math.PI / 5;
      points.push(Math.cos(outerAngle) * outer, Math.sin(outerAngle) * outer);
      points.push(Math.cos(innerAngle) * inner, Math.sin(innerAngle) * inner);
    }
    g.drawPolygon(points);
    g.endFill();
    return g;
  }

  static createMonsterIcon(color: number, size: number = 14): PIXI.Graphics {
    const g = new PIXI.Graphics();
    g.beginFill(color);
    // Diamond shape for monsters
    const s = size / 2;
    g.drawPolygon([0, -s, s, 0, 0, s, -s, 0]);
    g.endFill();
    return g;
  }
}
