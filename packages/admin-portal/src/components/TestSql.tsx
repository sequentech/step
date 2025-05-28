import React, { useState, useEffect } from 'react';
import initSqlJs, { Database, SqlJsStatic } from 'sql.js';

function LocalDatabaseLoader() {
    const [SQL, setSQL] = useState<SqlJsStatic | null>(null);
    const [db, setDb] = useState<Database | null>(null);
    const [isLoading, setIsLoading] = useState<boolean>(true);
    const [results, setResults] = useState<any[]>([]);
    const [tables, setTables] = useState<string[]>([]);

    useEffect(() => {
        const initializeSQL = async () => {
            try {
                // Initialize SQL.js
                const sql = await initSqlJs({
                    locateFile: file => `https://sql.js.org/dist/${file}`
                });
                setSQL(sql);

                // Load database from public folder
                await loadLocalDatabase(sql);
                
            } catch (error) {
                console.error('Failed to initialize SQL.js:', error);
                setIsLoading(false);
            }
        };

        initializeSQL();
    }, []);

    const loadLocalDatabase = async (sql: SqlJsStatic) => {
        try {
            // Load from public folder - adjust the filename as needed
            const response = await fetch('/results-a98ed291-5111-4201-915d-04adc4af157c.db');
            
            if (!response.ok) {
                throw new Error(`Failed to load database: ${response.statusText}`);
            }

            const buffer = await response.arrayBuffer();
            const uInt8Array = new Uint8Array(buffer);
            const database = new sql.Database(uInt8Array);
            
            setDb(database);
            loadTableList(database);
            setIsLoading(false);
            console.log('Local database loaded successfully!');
            
        } catch (error) {
            console.error('Error loading local database:', error);
            setIsLoading(false);
        }
    };

    // Alternative: Load different databases on demand
    const loadSpecificDatabase = async (filename: string) => {
        if (!SQL) return;

        try {
            const response = await fetch(`/${filename}`);
            if (!response.ok) {
                throw new Error(`Database file not found: ${filename}`);
            }

            const buffer = await response.arrayBuffer();
            const uInt8Array = new Uint8Array(buffer);
            const database = new SQL.Database(uInt8Array);
            
            setDb(database);
            loadTableList(database);
            console.log(`Database ${filename} loaded successfully!`);
            
        } catch (error) {
            console.error(`Error loading ${filename}:`, error);
        }
    };

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

    const executeQuery = (query: string) => {
        if (!db) return;

        try {
            const stmt = db.prepare(query);
            const queryResults: any[] = [];
            
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

    if (isLoading) return <div>Loading database from file system...</div>;

    return (
        <div style={{ padding: '20px', maxWidth: '1200px' }}>
            <h2>Local Database Loader</h2>
            
            {/* Quick Load Buttons */}
            <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                <h3>Load Pre-placed Databases</h3>
                <button 
                    onClick={() => loadSpecificDatabase('sample-database.sqlite')}
                    style={{ marginRight: '10px' }}
                >
                    Load Sample Database
                </button>
                <button 
                    onClick={() => loadSpecificDatabase('test.db')}
                    style={{ marginRight: '10px' }}
                >
                    Load Test Database
                </button>
                <button 
                    onClick={() => loadSpecificDatabase('production.sqlite')}
                >
                    Load Production Database
                </button>
            </div>

            {/* Database Info */}
            {db && (
                <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Database Info</h3>
                    <p><strong>Tables:</strong> {tables.join(', ')}</p>
                </div>
            )}

            {/* Quick Queries */}
            {db && tables.length > 0 && (
                <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Quick Table Views</h3>
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
            )}

            {/* Custom Query */}
            {db && (
                <div style={{ marginBottom: '20px', border: '1px solid #ccc', padding: '15px' }}>
                    <h3>Custom Query</h3>
                    <textarea
                        placeholder="Enter SQL query here..."
                        rows={4}
                        style={{ width: '100%', marginBottom: '10px' }}
                        onKeyPress={(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
                            if (e.key === 'Enter' && e.ctrlKey) {
                                const target = e.target as HTMLTextAreaElement;
                                executeQuery(target.value);
                            }
                        }}
                    />
                    <button onClick={(e) => {
                        const textarea = e.currentTarget.previousElementSibling as HTMLTextAreaElement;
                        executeQuery(textarea.value);
                    }}>
                        Execute Query
                    </button>
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

export default LocalDatabaseLoader;