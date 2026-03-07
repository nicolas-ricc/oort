import { ReactNode, RefObject } from "react"

type LayoutProps = {
    children: ReactNode;
    canvasRef?: RefObject<HTMLDivElement | null>;
}

export const Layout = ({ children, canvasRef }: LayoutProps) => {
    return (
        <div className="fixed inset-0 bg-slate-900">
            <div ref={canvasRef} className="h-full w-full relative">
                {children}
            </div>
        </div>
    )
}
