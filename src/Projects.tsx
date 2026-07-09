import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  deleteProject,
  detectCommand,
  listProjects,
  saveProject,
  startApp,
  type Project,
} from "./api";

// cwd 경로에서 폴더명 추출 (기본 이름 제안용)
function folderName(p: string): string {
  const parts = p.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || p;
}

// 간단 uuid (crypto.randomUUID는 tauri 웹뷰에서 사용 가능)
const newId = () => crypto.randomUUID();

export function Projects({
  onToast,
  suppressHide,
}: {
  onToast: (msg: string) => void;
  suppressHide: React.MutableRefObject<boolean>;
}) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState<Project | null>(null);
  const [starting, setStarting] = useState<string | null>(null);

  const refresh = useCallback(() => {
    listProjects()
      .then(setProjects)
      .catch(() => {});
  }, []);

  useEffect(refresh, [refresh]);

  // 폴더 선택 → 드래프트 생성(이름은 폴더명, 명령 기본 pnpm dev)
  const pickFolder = async () => {
    suppressHide.current = true; // 다이얼로그 중 팝오버가 숨지 않게
    let picked: string | string[] | null = null;
    try {
      picked = await open({ directory: true, multiple: false });
    } finally {
      // 다이얼로그 닫힌 뒤 잠깐 여유를 두고 플래그 해제(포커스 복귀 이벤트 지나가게)
      setTimeout(() => {
        suppressHide.current = false;
      }, 300);
    }
    if (typeof picked !== "string") return;

    // 폴더에서 패키지 매니저+스크립트 자동 감지 → 명령 자동 완성
    let command = "pnpm dev";
    try {
      const d = await detectCommand(picked);
      if (d.suggestedCommand) command = d.suggestedCommand;
    } catch {
      /* 감지 실패 시 기본값 유지 */
    }

    setDraft({
      id: newId(),
      name: folderName(picked),
      cwd: picked,
      command,
    });
    setAdding(true);
  };

  const submit = async () => {
    if (!draft) return;
    try {
      const updated = await saveProject(draft);
      setProjects(updated);
      setAdding(false);
      setDraft(null);
    } catch (e) {
      onToast(String(e));
    }
  };

  const run = async (p: Project) => {
    setStarting(p.id);
    try {
      await startApp(p.id);
      onToast(`${p.name} 실행됨`);
    } catch (e) {
      onToast(String(e));
    } finally {
      setStarting(null);
    }
  };

  const remove = async (p: Project) => {
    if (!window.confirm(`"${p.name}" 등록을 삭제할까요? (실행 중인 서버는 안 꺼져요)`))
      return;
    try {
      setProjects(await deleteProject(p.id));
    } catch (e) {
      onToast(String(e));
    }
  };

  return (
    <div className="projects">
      <div className="projects__head">
        <span>내 프로젝트</span>
        <button className="projects__add" onClick={pickFolder} title="폴더 추가">
          + 폴더
        </button>
      </div>

      {projects.length === 0 && !adding && (
        <p className="projects__empty">
          폴더를 등록하면 ▶ 버튼으로 dev 서버를 켤 수 있어요.
        </p>
      )}

      {projects.map((p) => (
        <div className="prow" key={p.id} title={`${p.cwd} · ${p.command}`}>
          <button
            className="prow__run"
            onClick={() => run(p)}
            disabled={starting === p.id}
            title={`${p.command} 실행`}
          >
            {starting === p.id ? "…" : "▶"}
          </button>
          <div className="prow__main">
            <span className="prow__name">{p.name}</span>
            <span className="prow__cmd">{p.command}</span>
          </div>
          <button
            className="prow__del"
            onClick={() => remove(p)}
            title="등록 삭제"
          >
            ✕
          </button>
        </div>
      ))}

      {adding && draft && (
        <div className="pform">
          <label>
            이름
            <input
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </label>
          <label>
            폴더
            <input value={draft.cwd} readOnly className="pform__ro" />
          </label>
          <label>
            실행 명령
            <input
              value={draft.command}
              onChange={(e) => setDraft({ ...draft, command: e.target.value })}
              placeholder="pnpm dev"
            />
          </label>
          <div className="pform__actions">
            <button
              onClick={() => {
                setAdding(false);
                setDraft(null);
              }}
            >
              취소
            </button>
            <button className="pform__save" onClick={submit}>
              등록
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
