import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import type { UnitView, WeaponView } from '@/types/game';
import { formatInches } from '@/utils/formatters';

export function UnitInfoPanel() {
  const gameState = useGameStore((s) => s.gameState);
  const selectedUnitId = useGameStore((s) => s.selectedUnitId);
  const hoveredUnitId = useGameStore((s) => s.hoveredUnitId);

  if (!gameState) return null;

  const unitId = selectedUnitId ?? hoveredUnitId;
  const unit = unitId != null ? gameState.units.find((u) => u.id === unitId) : null;

  if (!unit) {
    return (
      <Panel title="Unit Info">
        <p className="text-gray-500 text-sm">Select a unit to view its datasheet</p>
      </Panel>
    );
  }

  return (
    <Panel title={unit.name}>
      {/* Characteristics bar */}
      <div className="flex gap-1 mb-3">
        <StatBox label="M" value={formatInches(unit.movement)} />
        <StatBox label="T" value={`${unit.toughness}`} />
        <StatBox label="Sv" value={unit.armor_save} />
        {unit.invulnerable_save && <StatBox label="Inv" value={unit.invulnerable_save} />}
        <StatBox label="W" value={`${unit.total_wounds_remaining}`} />
        <StatBox label="Ld" value={`${unit.leadership}+`} />
        <StatBox label="OC" value={`${unit.oc}`} />
      </div>

      {/* Status indicators */}
      <div className="flex flex-wrap gap-1 mb-3">
        <StatusBadge label={unit.status} active />
        {unit.battle_shocked && <StatusBadge label="Battle-shocked" color="text-purple-400" active />}
        {unit.below_half_strength && <StatusBadge label="Below Half" color="text-yellow-400" active />}
        {unit.engagement_status === 'Engaged' && <StatusBadge label="Engaged" color="text-red-400" active />}
      </div>

      {/* Models alive */}
      <div className="text-xs text-gray-400 mb-3">
        Models: {unit.models_alive}/{unit.starting_model_count}
      </div>

      {/* Ranged Weapons */}
      {unit.models.some((m) => m.ranged_weapons.length > 0) && (
        <div className="mb-3">
          <h4 className="text-xs font-semibold text-gray-400 uppercase mb-1">Ranged Weapons</h4>
          <WeaponTable
            weapons={unit.models
              .flatMap((m) => m.ranged_weapons)
              .filter((w, i, arr) => arr.findIndex((x) => x.name === w.name) === i)}
          />
        </div>
      )}

      {/* Melee Weapons */}
      {unit.models.some((m) => m.melee_weapons.length > 0) && (
        <div className="mb-3">
          <h4 className="text-xs font-semibold text-gray-400 uppercase mb-1">Melee Weapons</h4>
          <WeaponTable
            weapons={unit.models
              .flatMap((m) => m.melee_weapons)
              .filter((w, i, arr) => arr.findIndex((x) => x.name === w.name) === i)}
          />
        </div>
      )}

      {/* Keywords */}
      <div className="flex flex-wrap gap-1 mb-2">
        {unit.keywords.map((kw) => (
          <span key={kw} className="px-1.5 py-0.5 bg-surface rounded text-xs text-gray-400">
            {kw}
          </span>
        ))}
      </div>

      {/* Wargear Abilities */}
      {unit.wargear_abilities.length > 0 && (
        <div className="mt-2">
          <h4 className="text-xs font-semibold text-gray-400 uppercase mb-1">Wargear Abilities</h4>
          {unit.wargear_abilities.map((wa) => (
            <div key={wa.name} className="text-xs text-gray-300 mb-1">
              <span className="font-semibold text-gray-200">{wa.name}: </span>
              {wa.description}
              {!wa.active && <span className="text-gray-500"> (inactive)</span>}
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat-box">
      <span className="text-[10px] text-gray-500 uppercase">{label}</span>
      <span className="text-sm font-semibold text-gray-200">{value}</span>
    </div>
  );
}

function StatusBadge({ label, color = 'text-gray-300', active }: { label: string; color?: string; active: boolean }) {
  if (!active) return null;
  return (
    <span className={`px-1.5 py-0.5 bg-surface rounded text-xs ${color}`}>
      {label}
    </span>
  );
}

function WeaponTable({ weapons }: { weapons: WeaponView[] }) {
  return (
    <table className="weapon-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Range</th>
          <th>A</th>
          <th>BS/WS</th>
          <th>S</th>
          <th>AP</th>
          <th>D</th>
        </tr>
      </thead>
      <tbody>
        {weapons.map((w) => (
          <tr key={w.id}>
            <td className="font-medium">
              {w.name}
              {w.abilities.length > 0 && (
                <span className="text-gray-500 text-[10px] ml-1">
                  [{w.abilities.join(', ')}]
                </span>
              )}
            </td>
            <td>{w.weapon_type === 'Melee' ? 'Melee' : formatInches(w.range)}</td>
            <td>{w.attacks}</td>
            <td>{w.skill}</td>
            <td>{w.strength}</td>
            <td>{w.ap}</td>
            <td>{w.damage}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
