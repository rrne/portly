import { open } from "@tauri-apps/plugin-dialog";

export function Settings({
  home,
  roots,
  onChange,
  onClose,
}: {
  home: string;
  roots: string[];
  onChange: (next: string[]) => void;
  onClose: () => void;
}) {
  const addRoot = async () => {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && !roots.includes(picked)) {
      onChange([...roots, picked]);
    }
  };

  const removeRoot = (r: string) => {
    onChange(roots.filter((x) => x !== r));
  };

  return (
    <div className="settings">
      <div className="settings__head">
        <span>내 프로젝트 폴더</span>
        <button className="settings__close" onClick={onClose} title="닫기">
          ✕
        </button>
      </div>

      <p className="settings__desc">
        여기 지정한 폴더 안에서 띄운 것만 "내 dev 서버"로 봅니다.
        {roots.length === 0 && (
          <>
            {" "}
            지금은 <code>{home || "~"}</code> 하위 전체를 봅니다.
          </>
        )}
      </p>

      {roots.map((r) => (
        <div key={r} className="settings__root" title={r}>
          <span className="settings__root-path">{r}</span>
          <button
            className="settings__root-remove"
            onClick={() => removeRoot(r)}
            title="제거"
          >
            ✕
          </button>
        </div>
      ))}

      <button className="settings__add" onClick={addRoot}>
        + 폴더 추가
      </button>
    </div>
  );
}
