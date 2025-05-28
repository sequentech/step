import React, { useState, useEffect, useRef } from 'react';
import initSqlJs, { Database, SqlJsStatic } from 'sql.js';

interface QueryResult {
    [key: string]: any;
}

interface ErrorResult {
    error: string;
}

type ResultType = QueryResult | ErrorResult;

function TestSql() {
    const [SQL, setSQL] = useState<SqlJsStatic | null>(null);
    const [db, setDb] = useState<Database | null>(null);
    const [isLoading, setIsLoading] = useState<boolean>(true);
    const [results, setResults] = useState<ResultType[]>([]);
    const [tables, setTables] = useState<string[]>([]);
    const fileInputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
        const initializeSQL = async () => {
            try {
                const sql = await initSqlJs({
                    locateFile: file => `https://sql.js.org/dist/${file}`
                });
                setSQL(sql);
                setIsLoading(false);
            } catch (error) {
                console.error('Failed to initialize SQL.js:', error);
                setIsLoading(false);
            }
        };

        initializeSQL();
    }, []);

    // Method 1: Load database from file upload
    const loadDatabaseFromFile = (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file || !SQL) return;

        const reader = new FileReader();
        reader.onload = (e: ProgressEvent<FileReader>) => {
            try {
                if (e.target?.result) {
                    const uInt8Array = new Uint8Array(e.target.result as ArrayBuffer);
                    const database = new SQL.Database(uInt8Array);
                    setDb(database);
                    loadTableList(database);
                    console.log('Database loaded successfully!');
                }
            } catch (error) {
                console.error('Error loading database:', error);
            }
        };
        reader.readAsArrayBuffer(file);
    };

    // Method 2: Load database from URL
    const loadDatabaseFromURL = async (url: string) => {
        if (!SQL) return;

        try {
            const response = await fetch(url);
            const buffer = await response.arrayBuffer();
            const uInt8Array = new Uint8Array(buffer);
            const database = new SQL.Database(uInt8Array);
            setDb(database);
            loadTableList(database);
            console.log('Database loaded from URL!');
        } catch (error) {
            console.error('Error loading database from URL:', error);
        }
    };

    // Method 3: Create database from SQL dump
    const loadDatabaseFromSQL = (sqlDump: string) => {
        if (!SQL) return;

        try {
            const database = new SQL.Database();
            // Execute the SQL dump
            database.exec(sqlDump);
            setDb(database);
            loadTableList(database);
            console.log('Database created from SQL dump!');
        } catch (error) {
            console.error('Error creating database from SQL:', error);
        }
    };

    // Method 4: Load from Base64 encoded database
    const loadDatabaseFromBase64 = (base64String: string) => {
        if (!SQL) return;

        try {
            const binaryString = atob(base64String);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            const database = new SQL.Database(bytes);
            setDb(database);
            loadTableList(database);
            console.log('Database loaded from Base64!');
        } catch (error) {
            console.error('Error loading database from Base64:', error);
        }
    };

    // Helper function to get list of tables
    const loadTableList = (database: Database) => {
        try {
            const stmt = database.prepare("SELECT name FROM sqlite_master WHERE type='table';");
            const tableList: string[] = [];
            while (stmt.step()) {
                const row = stmt.getAsObject();
                tableList.push(row.name as string);
            }
            stmt.free();
            setTables(tableList);
        } catch (error) {
            console.error('Error loading table list:', error);
        }
    };

    // Execute query on loaded database
    const executeQuery = (query: string) => {
        if (!db) return;

        try {
            const stmt = db.prepare(query);
            const queryResults: QueryResult[] = [];
            
            while (stmt.step()) {
                queryResults.push(stmt.getAsObject());
            }
            
            stmt.free();
            setResults(queryResults);
        } catch (error) {
            console.error('Query error:', error);
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            setResults([{ error: errorMessage }]);
        }
    };

    // Export current database
    const exportDatabase = () => {
        if (!db) return;

        try {
            const data = db.export();
            const blob = new Blob([data], { type: 'application/x-sqlite3' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'database.sqlite';
            a.click();
            URL.revokeObjectURL(url);
        } catch (error) {
            console.error('Export error:', error);
        }
    };

    if (isLoading) return <div>Loading SQL.js...</div>;

    return (
        <div style={{ padding: '20px', maxWidth: '1200px' }}>
            <h2>SQL.js Database Loader</h2>
            
            {/* Database Loading Options */}
            <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                <h3>Load Database</h3>
                
                {/* File Upload */}
                <div style={{ marginBottom: '10px' }}>
                    <label>Load from file (.sqlite, .db):</label>
                    <input
                        ref={fileInputRef}
                        type="file"
                        accept=".sqlite,.db,.sqlite3"
                        onChange={loadDatabaseFromFile}
                        style={{ marginLeft: '10px' }}
                    />
                </div>

                {/* URL Loading */}
                <div style={{ marginBottom: '10px' }}>
                    <label>Load from URL:</label>
                    <input
                        type="text"
                        placeholder="https://example.com/database.sqlite"
                        onKeyPress={(e: React.KeyboardEvent<HTMLInputElement>) => {
                            if (e.key === 'Enter') {
                                const target = e.target as HTMLInputElement;
                                loadDatabaseFromURL(target.value);
                            }
                        }}
                        style={{ marginLeft: '10px', width: '300px' }}
                    />
                </div>

                {/* Sample Database Buttons */}
                <div style={{ marginBottom: '10px' }}>
                    <button 
                        onClick={() => loadDatabaseFromURL('https://github.com/lerocha/chinook-database/blob/master/ChinookDatabase/DataSources/Chinook_Sqlite.sqlite')}
                        style={{ marginRight: '10px' }}
                    >
                        Load Sample Database (Chinook)
                    </button>
                    
                    <button onClick={() => {
                        const sampleSQL = `
                            CREATE TABLE customers (
                                id INTEGER PRIMARY KEY,
                                name TEXT NOT NULL,
                                email TEXT UNIQUE,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                            );
                            
                            INSERT INTO customers (name, email) VALUES 
                            ('John Doe', 'john@example.com'),
                            ('Jane Smith', 'jane@example.com'),
                            ('Bob Johnson', 'bob@example.com');
                            
                            CREATE TABLE orders (
                                id INTEGER PRIMARY KEY,
                                customer_id INTEGER,
                                total DECIMAL(10,2),
                                order_date DATETIME DEFAULT CURRENT_TIMESTAMP,
                                FOREIGN KEY (customer_id) REFERENCES customers(id)
                            );
                            
                            INSERT INTO orders (customer_id, total) VALUES 
                            (1, 99.99), (2, 149.50), (1, 75.25);
                        `;
                        loadDatabaseFromSQL(sampleSQL);
                    }}>
                        Create Sample Database
                    </button>
                </div>
            </div>

            {/* Database Info */}
            {db && (
                <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Database Info</h3>
                    <p><strong>Tables:</strong> {tables.join(', ')}</p>
                    <button onClick={exportDatabase}>Export Database</button>
                </div>
            )}

            {/* Query Interface */}
            {db && (
                <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Query Database</h3>
                    <div style={{ marginBottom: '10px' }}>
                        <textarea
                            placeholder="Enter SQL query here..."
                            rows={4}
                            style={{ width: '100%' }}
                            onKeyPress={(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
                                if (e.key === 'Enter' && e.ctrlKey) {
                                    const target = e.target as HTMLTextAreaElement;
                                    executeQuery(target.value);
                                }
                            }}
                        />
                    </div>
                    <div>
                        {tables.map(table => (
                            <button
                                key={table}
                                onClick={() => executeQuery(`SELECT * FROM ${table} LIMIT 10;`)}
                                style={{ marginRight: '5px', marginBottom: '5px' }}
                            >
                                View {table}
                            </button>
                        ))}
                    </div>
                </div>
            )}

            {/* Results Display */}
            {results.length > 0 && (
                <div style={{ border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Query Results ({results.length} rows)</h3>
                    <div style={{ maxHeight: '400px', overflow: 'auto' }}>
                        <pre style={{ background: '#f5f5f5', padding: '10px', fontSize: '12px' }}>
                            {JSON.stringify(results, null, 2)}
                        </pre>
                    </div>
                </div>
            )}
        </div>
    );
}

export default TestSql;
