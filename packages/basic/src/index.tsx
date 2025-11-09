
import React from "react"
import { createBrowserRouter, RouterProvider } from "react-router"
import ReactDOM from "react-dom/client"
import { App } from "./App"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

const Home = () => <div>Home Page</div>

const router = createBrowserRouter(
    [
        {
            path: "/",
            element: <App />,
            children: [
            {
                index: true,
                element: <Home />,
            },
        ],
        }
    ]
)

root.render(
    <React.StrictMode>
        <RouterProvider router={router} />
    </React.StrictMode>
)