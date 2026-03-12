import { useRef, useEffect, useCallback } from 'react';
import * as PIXI from 'pixi.js';
import { useGameStore } from '@/store/gameStore';
import type { GameView, DecisionSurfaceView } from '@/types/game';
import { CANVAS_WIDTH, CANVAS_HEIGHT, BOARD_BG_COLOR } from './constants';
import { BoardRenderer } from './BoardRenderer';
import { TerrainRenderer } from './TerrainRenderer';
import { DeploymentOverlay } from './DeploymentOverlay';
import { ObjectiveRenderer } from './ObjectiveRenderer';
import { UnitRenderer } from './UnitRenderer';
import { MovementPreview } from './MovementPreview';
import { AttackVisualization } from './AttackVisualization';
import { CameraController } from './CameraController';
import { InteractionLayer } from './InteractionLayer';

export function BattlefieldCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const appRef = useRef<PIXI.Application | null>(null);
  const worldRef = useRef<PIXI.Container | null>(null);
  const cameraRef = useRef<CameraController | null>(null);
  const renderersRef = useRef<{
    board: BoardRenderer;
    terrain: TerrainRenderer;
    deployment: DeploymentOverlay;
    objectives: ObjectiveRenderer;
    units: UnitRenderer;
    movement: MovementPreview;
    attacks: AttackVisualization;
    interaction: InteractionLayer;
  } | null>(null);

  const gameState = useGameStore((s) => s.gameState);
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const selectedUnitId = useGameStore((s) => s.selectedUnitId);
  const targetUnitId = useGameStore((s) => s.targetUnitId);
  const hoveredUnitId = useGameStore((s) => s.hoveredUnitId);
  const selectUnit = useGameStore((s) => s.selectUnit);
  const setTargetUnit = useGameStore((s) => s.setTargetUnit);
  const setHoveredUnit = useGameStore((s) => s.setHoveredUnit);
  const applyAction = useGameStore((s) => s.applyAction);

  // Initialize PixiJS
  useEffect(() => {
    if (!containerRef.current || appRef.current) return;

    const app = new PIXI.Application({
      width: CANVAS_WIDTH,
      height: CANVAS_HEIGHT,
      backgroundColor: BOARD_BG_COLOR,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });

    containerRef.current.appendChild(app.view as HTMLCanvasElement);
    appRef.current = app;

    // Create world container for camera transforms
    const world = new PIXI.Container();
    app.stage.addChild(world);
    worldRef.current = world;

    // Create layer containers
    const boardLayer = new PIXI.Container();
    const terrainLayer = new PIXI.Container();
    const deploymentLayer = new PIXI.Container();
    const objectiveLayer = new PIXI.Container();
    const movementLayer = new PIXI.Container();
    const attackLayer = new PIXI.Container();
    const unitLayer = new PIXI.Container();
    const interactionLayer = new PIXI.Container();

    world.addChild(boardLayer);
    world.addChild(terrainLayer);
    world.addChild(deploymentLayer);
    world.addChild(objectiveLayer);
    world.addChild(movementLayer);
    world.addChild(attackLayer);
    world.addChild(unitLayer);
    world.addChild(interactionLayer);

    // Create renderers
    const renderers = {
      board: new BoardRenderer(boardLayer),
      terrain: new TerrainRenderer(terrainLayer),
      deployment: new DeploymentOverlay(deploymentLayer),
      objectives: new ObjectiveRenderer(objectiveLayer),
      units: new UnitRenderer(unitLayer),
      movement: new MovementPreview(movementLayer),
      attacks: new AttackVisualization(attackLayer),
      interaction: new InteractionLayer(interactionLayer),
    };
    renderersRef.current = renderers;

    // Create camera controller
    const camera = new CameraController(world, app);
    camera.fitToBoard();
    cameraRef.current = camera;

    // Draw initial board grid
    renderers.board.draw();

    return () => {
      camera.destroy();
      app.destroy(true, { children: true, texture: true });
      appRef.current = null;
      worldRef.current = null;
      renderersRef.current = null;
      cameraRef.current = null;
    };
  }, []);

  // Update rendering when game state changes
  useEffect(() => {
    if (!renderersRef.current || !gameState) return;

    const r = renderersRef.current;
    r.terrain.update(gameState.board);
    r.deployment.update(gameState.board);
    r.objectives.update(gameState.board.objectives);
    r.units.update(
      gameState.units,
      gameState.players,
      selectedUnitId,
      targetUnitId,
      hoveredUnitId,
    );

    // Update interaction layer callbacks
    r.interaction.update(
      gameState,
      decisionSurface,
      cameraRef.current!,
      selectUnit,
      setTargetUnit,
      setHoveredUnit,
      applyAction,
    );
  }, [gameState, decisionSurface, selectedUnitId, targetUnitId, hoveredUnitId, selectUnit, setTargetUnit, setHoveredUnit, applyAction]);

  // Update movement/attack previews when selection changes
  useEffect(() => {
    if (!renderersRef.current || !gameState) return;

    const r = renderersRef.current;
    r.movement.update(gameState, decisionSurface, selectedUnitId);
    r.attacks.update(gameState, decisionSurface, selectedUnitId, targetUnitId);
  }, [gameState, decisionSurface, selectedUnitId, targetUnitId]);

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-hidden bg-surface"
      style={{ touchAction: 'none' }}
    />
  );
}
