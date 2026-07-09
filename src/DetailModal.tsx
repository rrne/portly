import { useEffect, useState } from "react";
import { processDetail } from "./api";
import type { DecoratedProcess, ProcessDetail } from "./types";

/** KB → 사람이 읽는 단위 (MB/GB) */
function fmtMem(kb: number): string {
  const mb = kb / 1024;
  if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
  return `${mb.toFixed(0)} MB`;
}

export function DetailModal({
  p,
  onClose,
}: {
  p: DecoratedProcess;
  onClose: () => void;
}) {
  const [detail, setDetail] = useState<ProcessDetail | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    processDetail(p.pid)
      .then(setDetail)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [p.pid]);

  return (
    <div className="modal__backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal__head">
          <div className="modal__title">
            {p.label || p.command}
            {p.framework && <span className="row__badge">{p.framework}</span>}
          </div>
          <button className="modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        {/* 어떤 서비스인지 (추론) */}
        <div className="modal__service">
          {p.framework ? (
            <>
              <span className="modal__service-label">추정 서비스</span>
              <span className="modal__service-val">{p.framework}</span>
              <span className="modal__service-note">
                명령줄·포트로 추론한 값이에요
              </span>
            </>
          ) : (
            <span className="modal__service-note">
              어떤 서비스인지 자동 추론하지 못했어요 (아래 명령줄 참고)
            </span>
          )}
        </div>

        <dl className="modal__grid">
          <dt>포트</dt>
          <dd>
            {p.port} <span className="modal__dim">{p.protocol}</span>
          </dd>

          <dt>메모리</dt>
          <dd>
            {loading ? (
              "…"
            ) : detail ? (
              <>
                {detail.memPercent.toFixed(1)}%{" "}
                <span className="modal__dim">({fmtMem(detail.rssKb)})</span>
              </>
            ) : (
              "—"
            )}
          </dd>

          <dt>PID</dt>
          <dd>{p.pid}</dd>

          <dt>사용자</dt>
          <dd>{p.user}</dd>

          <dt>주소</dt>
          <dd className="modal__mono">{p.address}</dd>

          {p.cwd && (
            <>
              <dt>폴더</dt>
              <dd className="modal__mono modal__break">{p.cwd}</dd>
            </>
          )}

          <dt>명령줄</dt>
          <dd className="modal__mono modal__break">
            {loading ? "…" : detail?.fullCommand || p.command}
          </dd>
        </dl>
      </div>
    </div>
  );
}
