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
import { WallRenderer } from './WallRenderer';
import { HatchwayRenderer } from './HatchwayRenderer';

export function BattlefieldCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const appRef = useRef<PIXI.Application | null>(null);
  const worldRef = useRef<PIXI.Container | null>(null);
  const cameraRef = useRef<CameraController | null>(null);
  const renderersRef = useRef<{
    board: BoardRenderer;
    terrain: TerrainRenderer;
    walls: WallRenderer;
    hatchways: HatchwayRenderer;
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
  const submitDeploy = useGameStore((s) => s.submitDeploy);
  const submitMove = useGameStore((s) => s.submitMove);

  // Handle clicks on empty board space for free movement/deployment
  const handleBoardClick = useCallback((boardX: number, boardY: number) => {
    const gs = useGameStore.getState().gameState;
    const selUnitId = useGameStore.getState().selectedUnitId;
    if (!gs) return;

    // Deployment: click on board to place selected undeployed unit
    if (gs.phase === 'PreBattle' && selUnitId != null) {
      const unit = gs.units.find((u) => u.id === selUnitId);
      if (unit && unit.status === 'Undeployed') {
        submitDeploy(selUnitId, boardX, boardY);
        return;
      }
    }

    // Deployment: if no unit selected, auto-select first undeployed unit for the decision owner
    if (gs.phase === 'PreBattle' && selUnitId == null) {
      const firstUndeployed = gs.units.find(
        (u) => u.owner === gs.decision_owner && u.status === 'Undeployed',
      );
      if (firstUndeployed) {
        selectUnit(firstUndeployed.id);
        submitDeploy(firstUndeployed.id, boardX, boardY);
        return;
      }
    }

    // Movement: click on board to move selected unit
    if (gs.phase === 'Movement' && selUnitId != null) {
      const unit = gs.units.find((u) => u.id === selUnitId);
      if (unit && unit.status === 'OnBattlefield' && !unit.turn_flags.has_moved && unit.owner === gs.decision_owner) {
        submitMove(selUnitId, boardX, boardY);
        return;
      }
    }

    // Otherwise deselect
    selectUnit(null);
  }, [submitDeploy, submitMove, selectUnit]);

  // Initialize PixiJS
  useEffect(() => {
    if (!containerRef.current || appRef.current) return;

    // Use container dimensions if available, fallback to canvas constants
    const container = containerRef.current;
    const initWidth = container.clientWidth || CANVAS_WIDTH;
    const initHeight = container.clientHeight || CANVAS_HEIGHT;

    const app = new PIXI.Application({
      width: initWidth,
      height: initHeight,
      backgroundColor: BOARD_BG_COLOR,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });

    container.appendChild(app.view as HTMLCanvasElement);
    appRef.current = app;

    // Create world container for camera transforms
    const world = new PIXI.Container();
    app.stage.addChild(world);
    worldRef.current = world;

    // Create layer containers
    const boardLayer = new PIXI.Container();
    const terrainLayer = new PIXI.Container();
    const wallLayer = new PIXI.Container();
    const hatchwayLayer = new PIXI.Container();
    const deploymentLayer = new PIXI.Container();
    const objectiveLayer = new PIXI.Container();
    const movementLayer = new PIXI.Container();
    const attackLayer = new PIXI.Container();
    const unitLayer = new PIXI.Container();
    const interactionLayer = new PIXI.Container();

    world.addChild(boardLayer);
    world.addChild(terrainLayer);
    world.addChild(wallLayer);
    world.addChild(deploymentLayer);
    world.addChild(objectiveLayer);
    world.addChild(hatchwayLayer);
    world.addChild(movementLayer);
    world.addChild(attackLayer);
    world.addChild(unitLayer);
    world.addChild(interactionLayer);

    // Create renderers
    const renderers = {
      board: new BoardRenderer(boardLayer),
      terrain: new TerrainRenderer(terrainLayer),
      walls: new WallRenderer(wallLayer),
      hatchways: new HatchwayRenderer(hatchwayLayer),
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

    // Responsive canvas resizing via ResizeObserver
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0 && appRef.current && cameraRef.current) {
          appRef.current.renderer.resize(width, height);
          cameraRef.current.fitToBoard();
        }
      }
    });
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
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

    // Update board dimensions from game state (handles BA 48x28 vs CP 44x30)
    if (gameState.board.width && gameState.board.height) {
      r.board.setBoardDimensions(gameState.board.width, gameState.board.height);
      if (cameraRef.current) {
        cameraRef.current.setBoardDimensions(gameState.board.width, gameState.board.height);
        cameraRef.current.fitToBoard();
      }
      r.board.draw(); // Redraw grid with new dimensions
    }

    r.terrain.update(gameState.board);
    r.walls.update(gameState.board);
    r.hatchways.update(gameState.board);
    r.deployment.update(gameState.board, gameState.phase, gameState.decision_owner);
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
      handleBoardClick,
    );
  }, [gameState, decisionSurface, selectedUnitId, targetUnitId, hoveredUnitId, selectUnit, setTargetUnit, setHoveredUnit, applyAction, handleBoardClick]);

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
