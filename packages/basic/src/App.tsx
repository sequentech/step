
import React from "react"
import { Outlet } from "react-router-dom"

export const App: React.FC = () => {
    return <div className="app-root">
        <b>Basic test</b>
        <Outlet />
    </div>
}