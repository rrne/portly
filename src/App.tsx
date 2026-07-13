import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  homeDir,
  installStatus,
  killPid,
  loadConfig,
  openPort,
  processMeta,
  revealInFinder,
  saveConfig,
  scanPorts,
  type InstallStatus,
} from "./api";
import { groupAll, guessMe, riskOf, type Risk } from "./grouping";
import { toFriendly } from "./friendly";
import type { DecoratedProcess, PortProcess, ProcessMeta } from "./types";
import { Settings } from "./Settings";
import { DetailModal } from "./DetailModal";
import { Projects } from "./Projects";
import "./App.css";

function App() {
  const [procs, setProcs] = useState<PortProcess[]>([]);
  const [metaByPid, setMetaByPid] = useState<Record<number, ProcessMeta>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [killing, setKilling] = useState<number | null>(null);
  const [showSystem, setShowSystem] = useState(false);
  const [showOther, setShowOther] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [detailFor, setDetailFor] = useState<DecoratedProcess | null>(null);

  // 필터 기준: 홈 경로 + 사용자가 지정한 프로젝트 루트들
  const [home, setHome] = useState("");
  const [roots, setRoots] = useState<string[]>([]);

  // 실행 위치(DMG/translocation이면 트레이가 죽어 무반응 → 이동 안내)
  const [install, setInstall] = useState<InstallStatus | null>(null);

  // OS 다이얼로그(폴더 선택)가 열려 있는 동안엔 "포커스 잃으면 hide"를 억제한다.
  // (다이얼로그가 포커스를 가져가면 팝오버가 숨어버려 다이얼로그까지 닫히는 충돌 방지)
  const suppressHide = useRef(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await scanPorts();
      setProcs(list);

      // cwd가 그룹(내 dev)을 결정하므로, root 아닌 pid의 메타(cwd)를 먼저 다 받는다.
      const targetPids = [
        ...new Set(list.filter((p) => p.user !== "root").map((p) => p.pid)),
      ];
      if (targetPids.length > 0) {
        const metas = await processMeta(targetPids);
        const map: Record<number, ProcessMeta> = {};
        for (const m of metas) map[m.pid] = m;
        setMetaByPid(map);
      } else {
        setMetaByPid({});
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // 설정 로드 (홈 경로 + 저장된 루트)
  useEffect(() => {
    homeDir()
      .then(setHome)
      .catch(() => {});
    loadConfig()
      .then((c) => setRoots(c.projectRoots ?? []))
      .catch(() => {});
    installStatus()
      .then(setInstall)
      .catch(() => {});
  }, []);

  const persistRoots = useCallback(async (next: string[]) => {
    setRoots(next);
    try {
      await saveConfig({ projectRoots: next });
    } catch {
      /* 저장 실패는 조용히 무시(다음 저장에서 복구) */
    }
  }, []);

  const onKill = useCallback(
    async (p: DecoratedProcess, risk: Risk) => {
      // 위험한 것(시스템/데이터)은 종료 전 한 번 더 확인
      if (risk !== "none") {
        const msg =
          risk === "system"
            ? `⚠️ "${p.label || p.command}" 은(는) 시스템 프로세스입니다.\n끄면 macOS가 불안정해질 수 있어요. 정말 종료할까요?`
            : `⚠️ "${p.label || p.command}" 은(는) DB/인프라입니다.\n저장 안 된 데이터가 날아갈 수 있어요. 정말 종료할까요?`;
        if (!window.confirm(msg)) return;
      }
      setKilling(p.pid);
      setToast(null);
      try {
        await killPid(p.pid, p.command, false); // SIGTERM + PID 재사용 방어
        setToast(`${p.label || p.command} (${p.port}) 종료됨`);
      } catch (e) {
        setToast(String(e));
      } finally {
        setKilling(null);
        await refresh(); // 즉시 갱신
      }
    },
    [refresh],
  );

  useEffect(() => {
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
      else if (!suppressHide.current) win.hide(); // 다이얼로그 중엔 숨기지 않음
    });
    refresh();
    return () => {
      unlisten.then((f) => f());
    };
  }, [refresh]);

  // 토스트 자동 사라짐
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 3000);
    return () => clearTimeout(t);
  }, [toast]);

  const grouped = useMemo(() => {
    const me = guessMe(procs);
    // pid→cwd 맵 (그룹 판정용)
    const cwdByPid: Record<number, string | undefined> = {};
    for (const [pid, m] of Object.entries(metaByPid)) {
      cwdByPid[Number(pid)] = m.cwd || undefined;
    }
    const g = groupAll(procs, me, cwdByPid, roots, home);
    const decorate = (p: DecoratedProcess): DecoratedProcess => {
      const meta = metaByPid[p.pid];
      if (!meta) return p;
      const f = toFriendly(meta);
      return { ...p, label: f.label, framework: f.framework, cwd: f.cwd };
    };
    return {
      dev: g.dev.map(decorate),
      other: g.other.map(decorate),
      system: g.system,
    };
  }, [procs, metaByPid, roots, home]);

  const isEmpty = procs.length === 0 && !loading && !error;

  return (
    <div className="popover">
      <header className="popover__header">
        <span className="popover__title">Portly</span>
        <span className="popover__hint">
          {loading ? "스캔 중…" : `${procs.length}개`}
        </span>
        <button
          className="popover__refresh"
          onClick={refresh}
          disabled={loading}
          title="새로고침"
        >
          ↻
        </button>
        <button
          className="popover__refresh"
          onClick={() => setShowSettings((v) => !v)}
          title="설정"
        >
          ⚙
        </button>
      </header>

      {install && install.location !== "ok" && (
        <div className="install-warn">
          <div className="install-warn__title">
            ⚠️ 잘못된 위치에서 실행 중
          </div>
          <p className="install-warn__body">
            {install.location === "dmg"
              ? "지금 DMG(디스크 이미지) 안에서 실행 중입니다. 이러면 메뉴바 아이콘이 반응하지 않을 수 있어요."
              : "macOS 보안(Gatekeeper)이 앱을 임시 위치로 옮겨 실행 중입니다. 메뉴바 아이콘이 반응하지 않을 수 있어요."}{" "}
            <b>응용 프로그램(Applications) 폴더로 옮긴 뒤</b> 다시 실행하세요.
          </p>
          <button
            className="install-warn__btn"
            onClick={() => revealInFinder(install.exePath)}
          >
            Finder에서 위치 열기
          </button>
        </div>
      )}

      {showSettings && (
        <Settings
          home={home}
          roots={roots}
          onChange={persistRoots}
          onClose={() => setShowSettings(false)}
          suppressHide={suppressHide}
        />
      )}

      <main className="popover__body">
        <Projects
          suppressHide={suppressHide}
          onToast={(msg) => {
            setToast(msg);
            // 서버 실행 직후 목록에 반영되도록 잠깐 뒤 재스캔
            setTimeout(refresh, 1500);
          }}
        />

        {error && <div className="state state--error">{error}</div>}
        {isEmpty && (
          <div className="state">
            <div className="state__icon">🔌</div>
            <p>열려 있는 포트가 없습니다.</p>
          </div>
        )}

        {grouped.dev.length > 0 && (
          <Section title="내 dev 서버" count={grouped.dev.length} accent>
            {grouped.dev.map((p) => (
              <ProcessRow
                key={rowKey(p)}
                p={p}
                risk={riskOf(p, "dev")}
                killing={killing === p.pid}
                onKill={onKill}
                onDetail={setDetailFor}
                onOpen={openPort}
              />
            ))}
          </Section>
        )}

        {grouped.other.length > 0 && (
          <button
            className="section__toggle"
            onClick={() => setShowOther((v) => !v)}
          >
            <span>{showOther ? "▾" : "▸"} 기타</span>
            <span className="section__count">{grouped.other.length}</span>
          </button>
        )}
        {showOther &&
          grouped.other.map((p) => (
            <ProcessRow
              key={rowKey(p)}
              p={p}
              risk={riskOf(p, "other")}
              killing={killing === p.pid}
              onKill={onKill}
              onDetail={setDetailFor}
              onOpen={openPort}
            />
          ))}

        {grouped.system.length > 0 && (
          <button
            className="section__toggle"
            onClick={() => setShowSystem((v) => !v)}
          >
            <span>{showSystem ? "▾" : "▸"} 시스템 ⚠️</span>
            <span className="section__count">{grouped.system.length}</span>
          </button>
        )}
        {showSystem &&
          grouped.system.map((p) => (
            <ProcessRow
              key={rowKey(p)}
              p={p}
              dim
              risk="system"
              killing={killing === p.pid}
              onKill={onKill}
              onDetail={setDetailFor}
            />
          ))}
      </main>

      {detailFor && (
        <DetailModal p={detailFor} onClose={() => setDetailFor(null)} />
      )}

      {toast && <div className="toast">{toast}</div>}

      <footer className="popover__footer">
        <span>P4 · kill</span>
      </footer>
    </div>
  );
}

function Section({
  title,
  count,
  accent,
  children,
}: {
  title: string;
  count: number;
  accent?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="section">
      <div className={`section__head ${accent ? "section__head--accent" : ""}`}>
        <span>{title}</span>
        <span className="section__count">{count}</span>
      </div>
      {children}
    </div>
  );
}

function ProcessRow({
  p,
  dim,
  risk = "none",
  killing,
  onKill,
  onDetail,
  onOpen,
}: {
  p: DecoratedProcess;
  dim?: boolean;
  risk?: Risk;
  killing?: boolean;
  onKill?: (p: DecoratedProcess, risk: Risk) => void;
  onDetail?: (p: DecoratedProcess) => void;
  onOpen?: (port: number) => void;
}) {
  const title = p.label || p.command;
  const subtitle = p.framework
    ? p.framework
    : p.cwd
      ? p.cwd
      : `${p.command} · ${p.user}`;

  const riskLabel =
    risk === "system"
      ? "시스템 프로세스 — 끄면 macOS 불안정 위험"
      : risk === "data"
        ? "DB/인프라 — 데이터 손실 위험"
        : "";

  return (
    <div className={`row ${dim ? "row--dim" : ""}`} title={p.cwd || p.command}>
      <span className="row__port">{p.port || "—"}</span>
      <div className="row__main">
        <span className="row__cmd">
          {risk !== "none" && (
            <span className="row__risk" title={riskLabel}>
              ⚠️
            </span>
          )}
          <span className="row__name">{title}</span>
          {p.framework && (
            <span className={`row__badge fw-${badgeKey(p.framework)}`}>
              {p.framework}
            </span>
          )}
          {onDetail && (
            <button
              className="row__info"
              onClick={(e) => {
                e.stopPropagation();
                onDetail(p);
              }}
              title="상세 정보"
              aria-label="상세 정보"
            >
              ⓘ
            </button>
          )}
        </span>
        <span className="row__meta">
          {p.label ? subtitle : `pid ${p.pid} · ${p.user} · ${p.protocol}`}
        </span>
      </div>
      <div className="row__actions">
        {onOpen && p.port > 0 && p.protocol === "TCP" && (
          <button
            className="row__open"
            onClick={() => onOpen(p.port)}
            title={`http://localhost:${p.port} 브라우저로 열기`}
          >
            열기
          </button>
        )}
        {onKill && (
          <button
            className={`row__kill ${risk !== "none" ? "row__kill--risky" : ""}`}
            onClick={() => onKill(p, risk)}
            disabled={killing}
            title={riskLabel || `포트 ${p.port} 종료 (SIGTERM)`}
          >
            {killing ? "…" : "종료"}
          </button>
        )}
      </div>
    </div>
  );
}

const rowKey = (p: PortProcess) => `${p.pid}-${p.port}-${p.protocol}`;

// 프레임워크명 → 배지 색 클래스 키
function badgeKey(fw: string): string {
  const f = fw.toLowerCase();
  if (f.includes("next")) return "next";
  if (f.includes("vite")) return "vite";
  if (f.includes("postgre") || f.includes("mysql") || f.includes("mongo"))
    return "db";
  if (f.includes("redis")) return "db";
  if (f.includes("python") || f.includes("django") || f.includes("flask"))
    return "py";
  if (f.includes("java") || f.includes("spring")) return "java";
  return "default";
}

export default App;
