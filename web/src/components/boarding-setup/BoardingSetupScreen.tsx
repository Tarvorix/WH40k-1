import { useEffect, useState } from 'react';
import { clsx } from 'clsx';
import { useGameStore } from '@/store/gameStore';
import { useBoardingSetupStore } from '@/store/boardingSetupStore';
import type { BoardingSetupStep } from '@/store/boardingSetupStore';
import type {
  BoardingFaction,
  BoardingDetachment,
  BoardingUnitDatasheet,
  BoardingEnhancement,
  BoardingMissionSummary,
  SelectedUnit,
} from '@/types/game';
import { Button } from '@/components/shared/Button';

// ===== Step indicator component =====

const STEP_LABELS: { step: BoardingSetupStep; label: string }[] = [
  { step: 'select_faction', label: 'Faction' },
  { step: 'select_detachment', label: 'Detachment' },
  { step: 'build_army', label: 'Army' },
  { step: 'select_enhancements', label: 'Enhancements' },
  { step: 'designate_warlord', label: 'Warlord' },
  { step: 'opponent_setup', label: 'Opponent' },
  { step: 'select_mission', label: 'Mission' },
  { step: 'ready', label: 'Ready' },
];

function stepIndex(step: BoardingSetupStep): number {
  return STEP_LABELS.findIndex((s) => s.step === step);
}

function StepIndicator({ currentStep }: { currentStep: BoardingSetupStep }) {
  const currentIdx = stepIndex(currentStep);
  return (
    <div className="flex items-center justify-center gap-1 mb-8">
      {STEP_LABELS.map((s, i) => (
        <div key={s.step} className="flex items-center gap-1">
          <div
            className={clsx(
              'h-1.5 rounded-full transition-colors',
              i <= currentIdx ? 'bg-accent' : 'bg-surface-lighter',
              i === currentIdx ? 'w-20' : 'w-10',
            )}
          />
        </div>
      ))}
    </div>
  );
}

// ===== Points bar component =====

function PointsBar({ current, max }: { current: number; max: number }) {
  const pct = Math.min((current / max) * 100, 100);
  const isOver = current > max;

  return (
    <div className="w-full">
      <div className="flex justify-between text-sm mb-1">
        <span className="text-gray-400">Points</span>
        <span className={clsx(isOver ? 'text-red-400 font-bold' : 'text-gray-200')}>
          {current} / {max}
        </span>
      </div>
      <div className="w-full bg-surface rounded-full h-2.5">
        <div
          className={clsx(
            'h-2.5 rounded-full transition-all duration-300',
            isOver ? 'bg-red-500' : 'bg-accent',
          )}
          style={{ width: `${Math.min(pct, 100)}%` }}
        />
      </div>
    </div>
  );
}

// ===== Select Faction Step =====

function SelectFactionStep() {
  const factions = useBoardingSetupStore((s) => s.factions);
  const selectFaction = useBoardingSetupStore((s) => s.selectFaction);
  const setScreen = useGameStore((s) => s.setScreen);

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Choose Your Faction
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Select a faction for Boarding Actions
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {factions.map((faction) => (
          <button
            key={faction.faction_keyword}
            onClick={() => selectFaction(faction)}
            className={clsx(
              'card-hover text-left p-5 group',
              faction.faction_keyword === 'ADEPTUS CUSTODES'
                ? 'hover:border-custodes-gold'
                : 'hover:border-worldeaters-red',
            )}
          >
            <h3
              className={clsx(
                'font-heading text-xl font-bold mb-1',
                faction.faction_keyword === 'ADEPTUS CUSTODES'
                  ? 'text-custodes-gold'
                  : 'text-worldeaters-red',
              )}
            >
              {faction.faction_name}
            </h3>
            <p className="text-gray-400 text-xs mb-3">{faction.faction_keyword}</p>
            <p className="text-gray-300 text-sm mb-3">{faction.army_rule_description}</p>
            <div className="text-xs bg-surface rounded px-3 py-2">
              <span className="text-gray-400 font-semibold">Army Rule: </span>
              <span className="text-gray-300">{faction.army_rule_name}</span>
            </div>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setScreen('menu')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Main Menu
        </button>
      </div>
    </div>
  );
}

// ===== Select Detachment Step =====

function SelectDetachmentStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const selectDetachment = useBoardingSetupStore((s) => s.selectDetachment);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction) return null;

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Choose Detachment
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Select a detachment for {playerFaction.faction_name}
      </p>
      <div className="grid grid-cols-1 gap-4 mb-6">
        {playerFaction.detachments.map((det) => (
          <button
            key={det.detachment_name}
            onClick={() => selectDetachment(det)}
            className="card-hover text-left p-5"
          >
            <h3 className="font-heading text-lg text-accent font-bold mb-1">
              {det.detachment_name}
            </h3>
            <div className="text-xs text-gray-400 mb-2">{det.detachment_rule_name}</div>
            <p className="text-gray-300 text-sm mb-3">{det.detachment_rule_description}</p>
            <div className="flex flex-wrap gap-2 mb-2">
              {det.stratagems.map((strat) => (
                <span
                  key={strat.name}
                  className="text-xs bg-surface rounded px-2 py-1 text-gray-400"
                >
                  {strat.name} ({strat.cp_cost}CP)
                </span>
              ))}
            </div>
            <div className="text-xs text-gray-500">
              {det.allowed_units.length} unit types available | {det.enhancements.length} enhancements
            </div>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setStep('select_faction')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Faction Select
        </button>
      </div>
    </div>
  );
}

// ===== Build Army Step =====

function BuildArmyStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const totalPoints = useBoardingSetupStore((s) => s.totalPoints);
  const addUnit = useBoardingSetupStore((s) => s.addUnit);
  const removeUnit = useBoardingSetupStore((s) => s.removeUnit);
  const updateUnitSize = useBoardingSetupStore((s) => s.updateUnitSize);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  const [expandedUnit, setExpandedUnit] = useState<string | null>(null);

  if (!playerFaction || !playerDetachment) return null;

  const MAX_POINTS = 500;

  // Get datasheets that are allowed in this detachment
  const allowedNames = new Set(playerDetachment.allowed_units.map((u) => u.datasheet_name));
  const availableDatasheets = playerFaction.datasheets.filter((ds) =>
    allowedNames.has(ds.name),
  );

  // Count how many of each datasheet are currently selected
  const unitCounts: Record<string, number> = {};
  for (const u of selectedUnits) {
    unitCounts[u.datasheet_name] = (unitCounts[u.datasheet_name] ?? 0) + 1;
  }

  // Check if a datasheet can be added
  const canAddDatasheet = (ds: BoardingUnitDatasheet): boolean => {
    const ref = playerDetachment.allowed_units.find(
      (u) => u.datasheet_name === ds.name,
    );
    if (!ref) return false;
    const currentCount = unitCounts[ds.name] ?? 0;
    if (ref.max_count !== null && currentCount >= ref.max_count) return false;
    // Check if adding the cheapest option would exceed points
    const cheapest = Math.min(...ds.points.map((p) => p.points));
    if (totalPoints + cheapest > MAX_POINTS) return false;
    return true;
  };

  const handleAddUnit = (ds: BoardingUnitDatasheet, sizeIndex: number) => {
    const pointsEntry = ds.points[sizeIndex];
    addUnit({
      datasheet_name: ds.name,
      model_count: pointsEntry.model_count,
      points: pointsEntry.points,
      wargear_selections: [],
    });
    setExpandedUnit(null);
  };

  const hasCharacters = selectedUnits.some((u) => {
    const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
    return ds?.is_character;
  });

  const canContinue = selectedUnits.length >= 1 && totalPoints <= MAX_POINTS;

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Build Your Army
      </h2>
      <p className="text-gray-400 text-center text-sm mb-4">
        {playerFaction.faction_name} - {playerDetachment.detachment_name}
      </p>

      <PointsBar current={totalPoints} max={MAX_POINTS} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
        {/* Left: Available units */}
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">Available Units</h3>
          <div className="space-y-2">
            {availableDatasheets.map((ds) => {
              const ref = playerDetachment.allowed_units.find(
                (u) => u.datasheet_name === ds.name,
              );
              const currentCount = unitCounts[ds.name] ?? 0;
              const atMax =
                ref?.max_count !== null &&
                ref?.max_count !== undefined &&
                currentCount >= ref.max_count;
              const isExpanded = expandedUnit === ds.name;

              return (
                <div
                  key={ds.name}
                  className="card"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-semibold text-gray-100 text-sm">
                          {ds.name}
                        </span>
                        {ds.is_character && (
                          <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded">
                            CHARACTER
                          </span>
                        )}
                        {ds.is_battleline && (
                          <span className="text-xs bg-phase-movement/20 text-phase-movement px-1.5 py-0.5 rounded">
                            BATTLELINE
                          </span>
                        )}
                        {ds.is_epic_hero && (
                          <span className="text-xs bg-phase-fight/20 text-phase-fight px-1.5 py-0.5 rounded">
                            EPIC HERO
                          </span>
                        )}
                      </div>
                      <div className="text-xs text-gray-400 mb-1">
                        {ds.points.map((p) => `${p.model_count} model${p.model_count !== 1 ? 's' : ''}: ${p.points}pts`).join(' | ')}
                      </div>
                      <div className="flex gap-2 text-xs text-gray-500">
                        <span>M{ds.profile.movement}</span>
                        <span>T{ds.profile.toughness}</span>
                        <span>Sv{ds.profile.save}</span>
                        <span>W{ds.profile.wounds}</span>
                        <span>OC{ds.profile.oc}</span>
                      </div>
                      {ref?.max_count !== null && (
                        <div className="text-xs text-gray-500 mt-1">
                          {currentCount}/{ref?.max_count} selected
                        </div>
                      )}
                    </div>
                    <div className="flex flex-col gap-1">
                      {ds.points.length === 1 ? (
                        <Button
                          size="sm"
                          variant="primary"
                          disabled={!canAddDatasheet(ds) || atMax}
                          onClick={() => handleAddUnit(ds, 0)}
                        >
                          Add
                        </Button>
                      ) : (
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={!canAddDatasheet(ds) || atMax}
                          onClick={() =>
                            setExpandedUnit(isExpanded ? null : ds.name)
                          }
                        >
                          {isExpanded ? 'Cancel' : 'Add...'}
                        </Button>
                      )}
                    </div>
                  </div>
                  {/* Size selector when expanded */}
                  {isExpanded && ds.points.length > 1 && (
                    <div className="mt-2 pt-2 border-t border-gray-700">
                      <div className="text-xs text-gray-400 mb-2">Select unit size:</div>
                      <div className="flex gap-2">
                        {ds.points.map((p, idx) => (
                          <Button
                            key={p.model_count}
                            size="sm"
                            variant="primary"
                            disabled={totalPoints + p.points > MAX_POINTS}
                            onClick={() => handleAddUnit(ds, idx)}
                          >
                            {p.model_count} models ({p.points}pts)
                          </Button>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Selected roster */}
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">
            Your Roster ({selectedUnits.length} unit{selectedUnits.length !== 1 ? 's' : ''})
          </h3>
          {selectedUnits.length === 0 ? (
            <div className="card text-center text-gray-500 py-8">
              No units selected yet. Add units from the left panel.
            </div>
          ) : (
            <div className="space-y-2">
              {selectedUnits.map((unit, idx) => {
                const ds = playerFaction.datasheets.find(
                  (d) => d.name === unit.datasheet_name,
                );
                return (
                  <div key={`${unit.datasheet_name}-${idx}`} className="card">
                    <div className="flex items-center justify-between">
                      <div>
                        <span className="font-semibold text-gray-100 text-sm">
                          {unit.datasheet_name}
                        </span>
                        <span className="text-xs text-gray-400 ml-2">
                          {unit.model_count} model{unit.model_count !== 1 ? 's' : ''}
                        </span>
                        {ds?.is_character && (
                          <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded ml-2">
                            CHARACTER
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-3">
                        <span className="text-accent font-mono text-sm">
                          {unit.points}pts
                        </span>
                        {/* Size changer if multiple sizes available */}
                        {ds && ds.points.length > 1 && (
                          <select
                            value={unit.model_count}
                            onChange={(e) => {
                              const newCount = Number(e.target.value);
                              const newEntry = ds.points.find(
                                (p) => p.model_count === newCount,
                              );
                              if (newEntry) {
                                updateUnitSize(idx, newEntry.model_count, newEntry.points);
                              }
                            }}
                            className="bg-surface border border-gray-600 text-gray-200 text-xs rounded px-1 py-0.5"
                          >
                            {ds.points.map((p) => (
                              <option key={p.model_count} value={p.model_count}>
                                {p.model_count} ({p.points}pts)
                              </option>
                            ))}
                          </select>
                        )}
                        <Button
                          size="sm"
                          variant="danger"
                          onClick={() => removeUnit(idx)}
                        >
                          Remove
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => setStep('select_detachment')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Detachment Select
        </button>
        <Button
          variant="primary"
          disabled={!canContinue}
          onClick={() => {
            // If there are characters, go to enhancements. Otherwise skip to warlord or opponent.
            if (hasCharacters) {
              setStep('select_enhancements');
            } else {
              setStep('opponent_setup');
            }
          }}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Select Enhancements Step =====

function SelectEnhancementsStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const enhancements = useBoardingSetupStore((s) => s.enhancements);
  const addEnhancement = useBoardingSetupStore((s) => s.addEnhancement);
  const removeEnhancement = useBoardingSetupStore((s) => s.removeEnhancement);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction || !playerDetachment) return null;

  const MAX_ENHANCEMENTS = 2;

  // All available enhancements from the detachment (engine provides the full set including universal)
  const allEnhancements: BoardingEnhancement[] = [
    ...playerDetachment.enhancements,
  ];

  // Character units that can receive enhancements
  const characterUnits = selectedUnits
    .map((u, idx) => ({ unit: u, index: idx }))
    .filter(({ unit }) => {
      const ds = playerFaction.datasheets.find((d) => d.name === unit.datasheet_name);
      return ds?.is_character;
    });

  // Enhancement names already assigned
  const assignedEnhancementNames = new Set(enhancements.map((e) => e.enhancement_name));
  // Unit indices that already have an enhancement
  const unitsWithEnhancements = new Set(enhancements.map((e) => e.unit_index));

  const canAssign = enhancements.length < MAX_ENHANCEMENTS;

  const handleAssign = (enhancementName: string, unitIndex: number) => {
    addEnhancement({ enhancement_name: enhancementName, unit_index: unitIndex });
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Enhancements
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Assign up to {MAX_ENHANCEMENTS} enhancements to CHARACTER units ({enhancements.length}/{MAX_ENHANCEMENTS} assigned)
      </p>

      {/* Current assignments */}
      {enhancements.length > 0 && (
        <div className="mb-6">
          <h3 className="font-heading text-lg text-gray-300 mb-3">Current Assignments</h3>
          <div className="space-y-2">
            {enhancements.map((enh, idx) => {
              const unit = selectedUnits[enh.unit_index];
              return (
                <div key={idx} className="card flex items-center justify-between">
                  <div>
                    <span className="text-accent font-semibold text-sm">{enh.enhancement_name}</span>
                    <span className="text-gray-400 text-xs mx-2">assigned to</span>
                    <span className="text-gray-200 text-sm">{unit?.datasheet_name ?? 'Unknown'}</span>
                  </div>
                  <Button size="sm" variant="danger" onClick={() => removeEnhancement(idx)}>
                    Remove
                  </Button>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Enhancement list */}
      {canAssign && (
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">Available Enhancements</h3>
          <div className="space-y-3">
            {allEnhancements.map((enh) => {
              const isAssigned = assignedEnhancementNames.has(enh.name);
              return (
                <div
                  key={enh.name}
                  className={clsx('card', isAssigned && 'opacity-50')}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1">
                      <h4 className="font-semibold text-accent text-sm mb-1">{enh.name}</h4>
                      <p className="text-gray-300 text-xs">{enh.description}</p>
                    </div>
                    {!isAssigned && (
                      <div className="flex flex-col gap-1 min-w-[120px]">
                        <div className="text-xs text-gray-400 mb-1">Assign to:</div>
                        {characterUnits.map(({ unit, index: unitIdx }) => {
                          const alreadyHas = unitsWithEnhancements.has(unitIdx);
                          return (
                            <Button
                              key={unitIdx}
                              size="sm"
                              variant="secondary"
                              disabled={alreadyHas}
                              onClick={() => handleAssign(enh.name, unitIdx)}
                            >
                              {unit.datasheet_name}
                            </Button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => setStep('build_army')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Army Builder
        </button>
        <Button
          variant="primary"
          onClick={() => setStep('designate_warlord')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Designate Warlord Step =====

function DesignateWarlordStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const warlordIndex = useBoardingSetupStore((s) => s.warlordIndex);
  const setWarlord = useBoardingSetupStore((s) => s.setWarlord);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction) return null;

  // Character units eligible to be warlord
  const characterUnits = selectedUnits
    .map((u, idx) => ({ unit: u, index: idx }))
    .filter(({ unit }) => {
      const ds = playerFaction.datasheets.find((d) => d.name === unit.datasheet_name);
      return ds?.is_character;
    });

  // If no characters, auto-select first unit
  const eligibleUnits = characterUnits.length > 0
    ? characterUnits
    : selectedUnits.map((u, idx) => ({ unit: u, index: idx }));

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Designate Warlord
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        {characterUnits.length > 0
          ? 'Select a CHARACTER to be your Warlord'
          : 'Select a unit leader to be your Warlord'}
      </p>

      <div className="space-y-2 max-w-lg mx-auto mb-6">
        {eligibleUnits.map(({ unit, index: unitIdx }) => {
          const ds = playerFaction.datasheets.find(
            (d) => d.name === unit.datasheet_name,
          );
          const isSelected = warlordIndex === unitIdx;
          return (
            <button
              key={unitIdx}
              onClick={() => setWarlord(unitIdx)}
              className={clsx(
                isSelected ? 'card-selected' : 'card-hover',
                'w-full text-left',
              )}
            >
              <div className="flex items-center justify-between">
                <div>
                  <span className="font-semibold text-gray-100 text-sm">
                    {unit.datasheet_name}
                  </span>
                  <span className="text-xs text-gray-400 ml-2">
                    {unit.model_count} model{unit.model_count !== 1 ? 's' : ''}
                  </span>
                  {ds?.is_character && (
                    <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded ml-2">
                      CHARACTER
                    </span>
                  )}
                </div>
                {isSelected && (
                  <span className="text-accent font-heading text-xs tracking-wide">
                    WARLORD
                  </span>
                )}
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => {
            // Go back based on whether we had characters for enhancements
            const hasChars = selectedUnits.some((u) => {
              const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
              return ds?.is_character;
            });
            setStep(hasChars ? 'select_enhancements' : 'build_army');
          }}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back
        </button>
        <Button
          variant="primary"
          disabled={warlordIndex === null}
          onClick={() => setStep('opponent_setup')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Opponent Setup Step =====

function OpponentSetupStep() {
  const factions = useBoardingSetupStore((s) => s.factions);
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const opponentFaction = useBoardingSetupStore((s) => s.opponentFaction);
  const opponentDetachment = useBoardingSetupStore((s) => s.opponentDetachment);
  const setOpponent = useBoardingSetupStore((s) => s.setOpponent);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  // Auto-select opponent on mount if not already set
  useEffect(() => {
    if (!opponentFaction && playerFaction) {
      // Pick the other faction by default
      const other = factions.find(
        (f) => f.faction_keyword !== playerFaction.faction_keyword,
      );
      if (other && other.detachments.length > 0) {
        setOpponent(other, other.detachments[0]);
      }
    }
  }, [opponentFaction, playerFaction, factions, setOpponent]);

  const handleSelectOpponent = (faction: BoardingFaction) => {
    if (faction.detachments.length > 0) {
      setOpponent(faction, faction.detachments[0]);
    }
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Opponent Setup
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        The AI will command the opposing force. Select the opponent faction.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mb-6">
        {factions.map((faction) => {
          const isSelected =
            opponentFaction?.faction_keyword === faction.faction_keyword;
          return (
            <button
              key={faction.faction_keyword}
              onClick={() => handleSelectOpponent(faction)}
              className={clsx(
                isSelected ? 'card-selected' : 'card-hover',
                'text-left p-5',
              )}
            >
              <h3
                className={clsx(
                  'font-heading text-lg font-bold mb-1',
                  faction.faction_keyword === 'ADEPTUS CUSTODES'
                    ? 'text-custodes-gold'
                    : 'text-worldeaters-red',
                )}
              >
                {faction.faction_name}
              </h3>
              <p className="text-gray-400 text-xs mb-2">{faction.army_rule_name}</p>
              {isSelected && opponentDetachment && (
                <div className="text-xs text-gray-300 bg-surface rounded px-2 py-1 mt-2">
                  Detachment: {opponentDetachment.detachment_name}
                </div>
              )}
            </button>
          );
        })}
      </div>

      {/* Detachment override if the opponent faction has multiple */}
      {opponentFaction && opponentFaction.detachments.length > 1 && (
        <div className="max-w-lg mx-auto mb-6">
          <h3 className="text-sm text-gray-400 mb-2">Opponent Detachment:</h3>
          <div className="flex gap-2 flex-wrap">
            {opponentFaction.detachments.map((det) => (
              <button
                key={det.detachment_name}
                onClick={() => setOpponent(opponentFaction, det)}
                className={clsx(
                  'text-sm px-3 py-1.5 rounded',
                  opponentDetachment?.detachment_name === det.detachment_name
                    ? 'bg-accent text-black font-semibold'
                    : 'bg-surface-lighter text-gray-300 hover:bg-surface-light',
                )}
              >
                {det.detachment_name}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => {
            const hasChars = playerFaction
              ? (useBoardingSetupStore.getState().selectedUnits).some((u) => {
                  const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
                  return ds?.is_character;
                })
              : false;
            setStep(hasChars ? 'designate_warlord' : 'build_army');
          }}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back
        </button>
        <Button
          variant="primary"
          disabled={!opponentFaction || !opponentDetachment}
          onClick={() => setStep('select_mission')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Select Mission Step =====

function SelectMissionStep() {
  const missions = useBoardingSetupStore((s) => s.missions);
  const selectMission = useBoardingSetupStore((s) => s.selectMission);
  const selectedMission = useBoardingSetupStore((s) => s.selectedMission);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  const [filterType, setFilterType] = useState<string | null>(null);
  const [filterTag, setFilterTag] = useState<string | null>(null);

  // Collect all unique tags
  const allTags = Array.from(
    new Set(missions.flatMap((m) => m.tags)),
  ).sort();

  // Filter missions
  const filteredMissions = missions.filter((m) => {
    if (filterType && m.mission_type !== filterType) return false;
    if (filterTag && !m.tags.includes(filterTag)) return false;
    return true;
  });

  const handleSelect = (mission: BoardingMissionSummary) => {
    selectMission(mission);
    setStep('ready');
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Mission
      </h2>
      <p className="text-gray-400 text-center text-sm mb-4">
        Choose one of the 15 Boarding Actions missions
      </p>

      {/* Filters */}
      <div className="flex flex-wrap gap-2 justify-center mb-6">
        <button
          onClick={() => setFilterType(null)}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === null
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          All Types
        </button>
        <button
          onClick={() => setFilterType('symmetric')}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === 'symmetric'
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          Symmetric
        </button>
        <button
          onClick={() => setFilterType('asymmetric')}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === 'asymmetric'
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          Asymmetric
        </button>
        <span className="text-gray-600 mx-1">|</span>
        <button
          onClick={() => setFilterTag(null)}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterTag === null
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          All Tags
        </button>
        {allTags.map((tag) => (
          <button
            key={tag}
            onClick={() => setFilterTag(filterTag === tag ? null : tag)}
            className={clsx(
              'text-xs px-3 py-1 rounded-full border capitalize',
              filterTag === tag
                ? 'bg-accent text-black border-accent font-semibold'
                : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
            )}
          >
            {tag}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 mb-6">
        {filteredMissions.map((mission) => (
          <button
            key={mission.mission_id}
            onClick={() => handleSelect(mission)}
            className={clsx(
              selectedMission?.mission_id === mission.mission_id
                ? 'card-selected'
                : 'card-hover',
              'text-left p-4',
            )}
          >
            <div className="flex items-start justify-between mb-2">
              <h3 className="font-semibold text-accent text-sm">{mission.name}</h3>
              <span className="text-xs text-gray-500 font-mono">{mission.mission_id}</span>
            </div>
            <div className="flex gap-2 flex-wrap">
              <span
                className={clsx(
                  'text-xs px-2 py-0.5 rounded',
                  mission.mission_type === 'symmetric'
                    ? 'bg-phase-command/20 text-phase-command'
                    : 'bg-phase-charge/20 text-phase-charge',
                )}
              >
                {mission.mission_type}
              </span>
              {mission.tags.map((tag) => (
                <span
                  key={tag}
                  className="text-xs bg-surface text-gray-400 px-2 py-0.5 rounded capitalize"
                >
                  {tag}
                </span>
              ))}
            </div>
          </button>
        ))}
      </div>

      <div className="text-center">
        <button
          onClick={() => setStep('opponent_setup')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Opponent Setup
        </button>
      </div>
    </div>
  );
}

// ===== Ready Step =====

function ReadyStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const totalPoints = useBoardingSetupStore((s) => s.totalPoints);
  const enhancements = useBoardingSetupStore((s) => s.enhancements);
  const warlordIndex = useBoardingSetupStore((s) => s.warlordIndex);
  const opponentFaction = useBoardingSetupStore((s) => s.opponentFaction);
  const opponentDetachment = useBoardingSetupStore((s) => s.opponentDetachment);
  const selectedMission = useBoardingSetupStore((s) => s.selectedMission);
  const setStep = useBoardingSetupStore((s) => s.setStep);
  const reset = useBoardingSetupStore((s) => s.reset);

  const engineReady = useGameStore((s) => s.engineReady);
  const loading = useGameStore((s) => s.loading);
  const createBoardingMatch = useGameStore((s) => s.createBoardingMatch);

  if (!playerFaction || !playerDetachment || !selectedMission) return null;

  const handleStartBattle = async () => {
    // Parse faction IDs from the faction data
    // The engine uses numeric faction IDs; we derive them from the faction index in the loaded list
    const setupState = useBoardingSetupStore.getState();
    const factions = setupState.factions;
    const playerFactionIndex = factions.findIndex(
      (f) => f.faction_name === playerFaction.faction_name &&
             f.detachments.some((d) => d.detachment_name === playerDetachment!.detachment_name)
    );
    const opponentFactionIndex = factions.findIndex(
      (f) => f.faction_name === opponentFaction?.faction_name &&
             f.detachments.some((d) => d.detachment_name === opponentDetachment?.detachment_name)
    );

    const missionIdNum = parseInt(selectedMission.mission_id.replace(/\D/g, ''), 10) || 1;

    // Serialize the player's roster selections for the engine
    const playerRoster = {
      faction_name: playerFaction.faction_name,
      faction_keyword: playerFaction.faction_keyword,
      detachment_name: playerDetachment!.detachment_name,
      units: setupState.selectedUnits.map((u) => ({
        datasheet_name: u.datasheet_name,
        model_count: u.model_count,
        points: u.points,
        wargear_selections: u.wargear_selections ?? [],
      })),
      warlord_index: setupState.warlordIndex,
      enhancements: setupState.enhancements.map((e) => ({
        enhancement_name: e.enhancement_name,
        unit_index: e.unit_index,
      })),
      total_points: setupState.totalPoints,
    };

    await createBoardingMatch({
      playerFactionId: playerFactionIndex >= 0 ? playerFactionIndex : 0,
      opponentFactionId: opponentFactionIndex >= 0 ? opponentFactionIndex : 1,
      missionId: missionIdNum,
      playerRoster,
    });
  };

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-6">
        Ready to Battle
      </h2>

      <div className="card mb-6">
        <h3 className="font-heading text-lg text-accent mb-4">Battle Summary</h3>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-gray-400">Your Faction:</span>
            <span className="ml-2 text-gray-100">{playerFaction.faction_name}</span>
          </div>
          <div>
            <span className="text-gray-400">Opponent:</span>
            <span className="ml-2 text-gray-100">{opponentFaction?.faction_name ?? 'N/A'}</span>
          </div>
          <div>
            <span className="text-gray-400">Detachment:</span>
            <span className="ml-2 text-gray-100">{playerDetachment.detachment_name}</span>
          </div>
          <div>
            <span className="text-gray-400">Opp. Detachment:</span>
            <span className="ml-2 text-gray-100">{opponentDetachment?.detachment_name ?? 'N/A'}</span>
          </div>
          <div>
            <span className="text-gray-400">Mission:</span>
            <span className="ml-2 text-gray-100">{selectedMission.name}</span>
          </div>
          <div>
            <span className="text-gray-400">Points:</span>
            <span className="ml-2 text-gray-100">{totalPoints} / 500</span>
          </div>
        </div>
      </div>

      {/* Roster */}
      <div className="card mb-6">
        <h3 className="font-heading text-lg text-accent mb-3">Your Roster</h3>
        <div className="space-y-2">
          {selectedUnits.map((unit, idx) => {
            const isWarlord = warlordIndex === idx;
            const unitEnhancements = enhancements.filter((e) => e.unit_index === idx);
            return (
              <div
                key={`${unit.datasheet_name}-${idx}`}
                className="flex items-center justify-between text-sm py-1 border-b border-gray-700 last:border-0"
              >
                <div className="flex items-center gap-2">
                  <span className="text-gray-100">{unit.datasheet_name}</span>
                  <span className="text-xs text-gray-500">
                    x{unit.model_count}
                  </span>
                  {isWarlord && (
                    <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded">
                      WARLORD
                    </span>
                  )}
                  {unitEnhancements.map((e) => (
                    <span
                      key={e.enhancement_name}
                      className="text-xs bg-phase-prebattle/20 text-phase-prebattle px-1.5 py-0.5 rounded"
                    >
                      {e.enhancement_name}
                    </span>
                  ))}
                </div>
                <span className="text-accent font-mono text-xs">{unit.points}pts</span>
              </div>
            );
          })}
        </div>
      </div>

      <div className="flex gap-4 justify-center">
        <Button variant="secondary" onClick={() => reset()}>
          Start Over
        </Button>
        <Button variant="secondary" onClick={() => setStep('select_mission')}>
          Change Mission
        </Button>
        <Button
          variant="primary"
          size="lg"
          onClick={handleStartBattle}
          disabled={!engineReady || loading}
        >
          {loading ? 'Creating Match...' : 'Start Battle'}
        </Button>
      </div>
    </div>
  );
}

// ===== Main Boarding Setup Screen =====

export function BoardingSetupScreen() {
  const step = useBoardingSetupStore((s) => s.step);
  const setFactions = useBoardingSetupStore((s) => s.setFactions);
  const setMissions = useBoardingSetupStore((s) => s.setMissions);

  const [dataLoading, setDataLoading] = useState(true);
  const [dataError, setDataError] = useState<string | null>(null);
  const engineReady = useGameStore((s) => s.engineReady);

  useEffect(() => {
    if (!engineReady) return;

    const loadData = async () => {
      try {
        setDataLoading(true);
        setDataError(null);

        const { engineClient } = await import('@/engine/workerClient');
        const factions = await engineClient.getBoardingFactions();
        const missions = await engineClient.getBoardingMissions();

        // Map mission packages to summaries for the UI
        const missionSummaries: BoardingMissionSummary[] = missions.map((m: any) => ({
          mission_id: m.mission_id ?? m.id ?? '',
          name: m.name ?? m.mission_name ?? '',
          mission_type: m.mission_type ?? 'symmetric',
          tags: m.tags ?? [],
        }));

        setFactions(factions);
        setMissions(missionSummaries);
        setDataLoading(false);
      } catch (err) {
        console.error('[BoardingSetup] Failed to load data from engine:', err);
        setDataError(err instanceof Error ? err.message : 'Failed to load boarding data');
        setDataLoading(false);
      }
    };

    loadData();
  }, [engineReady, setFactions, setMissions]);

  return (
    <div className="flex-1 flex flex-col items-center justify-start p-8 overflow-y-auto">
      <div className="max-w-4xl w-full">
        <h1 className="font-heading text-4xl text-accent font-bold text-center mb-2">
          BOARDING ACTIONS
        </h1>
        <p className="text-gray-400 text-center mb-4 text-sm">
          Ship Interior Combat - Army Setup
        </p>

        <StepIndicator currentStep={step} />

        {dataLoading && (
          <div className="text-center text-gray-400 py-12">Loading boarding actions data from engine...</div>
        )}
        {dataError && (
          <div className="text-center text-red-400 py-12">Error: {dataError}</div>
        )}
        {!dataLoading && !dataError && (
          <>
            {step === 'select_faction' && <SelectFactionStep />}
            {step === 'select_detachment' && <SelectDetachmentStep />}
            {step === 'build_army' && <BuildArmyStep />}
            {step === 'select_enhancements' && <SelectEnhancementsStep />}
            {step === 'designate_warlord' && <DesignateWarlordStep />}
            {step === 'opponent_setup' && <OpponentSetupStep />}
            {step === 'select_mission' && <SelectMissionStep />}
            {step === 'ready' && <ReadyStep />}
          </>
        )}
      </div>
    </div>
  );
}
