// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// hooks/useSQLiteDatabase.ts
import { useState, useEffect, useCallback, useRef, createContext, useContext } from 'react';
import initSqlJs, { Database, SqlJsStatic } from 'sql.js';

interface QueryResult {
    [key: string]: any;
}

interface DatabaseState {
    isLoading: boolean;
    isReady: boolean;
    error: string | null;
    tables: string[];
}

interface QueryOptions {
    enableCache?: boolean;
    cacheKey?: string;
}

interface CachedQuery {
    data: QueryResult[];
    timestamp: number;
    ttl: number; // time to live in milliseconds
}

// Context for global database management
interface DatabaseContextType {
    databases: Map<string, Database>;
    defaultDatabase?: string;
    addDatabase: (name: string, database: Database) => void;
    removeDatabase: (name: string) => void;
    setDefaultDatabase: (name: string) => void;
}

const DatabaseContext = createContext<DatabaseContextType | null>(null);

// Hook to use the database context
export function useDatabaseContext() {
    const context = useContext(DatabaseContext);
    if (!context) {
        throw new Error('useDatabaseContext must be used within a DatabaseProvider');
    }
    return context;
}

// Hook to create database context value (for use in your provider component)
export function useDatabaseManager() {
    const [databases] = useState(new Map<string, Database>());
    const [defaultDatabase, setDefaultDb] = useState<string>();

    const addDatabase = useCallback((name: string, database: Database) => {
        databases.set(name, database);
        if (!defaultDatabase) {
            setDefaultDb(name);
        }
    }, [databases, defaultDatabase]);

    const removeDatabase = useCallback((name: string) => {
        databases.delete(name);
        if (defaultDatabase === name) {
            const remaining = Array.from(databases.keys());
            setDefaultDb(remaining.length > 0 ? remaining[0] : undefined);
        }
    }, [databases, defaultDatabase]);

    const setDefaultDatabase = useCallback((name: string) => {
        if (databases.has(name)) {
            setDefaultDb(name);
        }
    }, [databases]);

    return {
        databases,
        defaultDatabase,
        addDatabase,
        removeDatabase,
        setDefaultDatabase,
        contextValue: {
            databases,
            defaultDatabase,
            addDatabase,
            removeDatabase,
            setDefaultDatabase
        }
    };
}

// Export the context for use in provider
export { DatabaseContext };

// Main hook for SQLite database operations
export function useSQLiteDatabase(databaseUrl?: string) {
    const [SQL, setSQL] = useState<SqlJsStatic | null>(null);
    const [db, setDb] = useState<Database | null>(null);
    const [state, setState] = useState<DatabaseState>({
        isLoading: true,
        isReady: false,
        error: null,
        tables: []
    });

    // Cache for query results
    const queryCache = useRef<Map<string, CachedQuery>>(new Map());
    const defaultCacheTTL = 5 * 60 * 1000; // 5 minutes

    // Initialize SQL.js and database
    useEffect(() => {
        const initializeDatabase = async () => {
            try {
                setState(prev => ({ ...prev, isLoading: true, error: null }));

                // Initialize SQL.js
                const sql = await initSqlJs({
                    locateFile: file => `https://sql.js.org/dist/${file}`
                });
                setSQL(sql);

                // Load database if URL provided
                if (databaseUrl) {
                    await loadDatabase(sql, databaseUrl);
                } else {
                    // Create empty database
                    const database = new sql.Database();
                    setDb(database);
                    setState(prev => ({ 
                        ...prev, 
                        isLoading: false, 
                        isReady: true, 
                        tables: [] 
                    }));
                }
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Unknown error';
                setState(prev => ({ 
                    ...prev, 
                    isLoading: false, 
                    error: errorMessage 
                }));
            }
        };

        initializeDatabase();
    }, [databaseUrl]);

    // Load database from URL
    const loadDatabase = async (sql: SqlJsStatic, url: string) => {
        try {
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`Failed to load database: ${response.statusText}`);
            }

            const buffer = await response.arrayBuffer();
            const uInt8Array = new Uint8Array(buffer);
            const database = new sql.Database(uInt8Array);
            
            setDb(database);
            const tables = await getTableList(database);
            
            setState(prev => ({ 
                ...prev, 
                isLoading: false, 
                isReady: true, 
                tables 
            }));

            // Clear cache when new database is loaded
            queryCache.current.clear();
            
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            setState(prev => ({ 
                ...prev, 
                isLoading: false, 
                error: errorMessage 
            }));
        }
    };

    // Get list of tables
    const getTableList = async (database: Database): Promise<string[]> => {
        try {
            const stmt = database.prepare("SELECT name FROM sqlite_master WHERE type='table';");
            const tables: string[] = [];
            while (stmt.step()) {
                const row = stmt.getAsObject();
                tables.push(row.name as string);
            }
            stmt.free();
            return tables;
        } catch (error) {
            console.error('Error getting table list:', error);
            return [];
        }
    };

    // Check if cached query is still valid
    const isCacheValid = (cached: CachedQuery): boolean => {
        return Date.now() - cached.timestamp < cached.ttl;
    };

    // Execute SQL query with caching support
    const query = useCallback(async (
        sql: string, 
        params: any[] = [], 
        options: QueryOptions = {}
    ): Promise<QueryResult[]> => {
        if (!db) {
            throw new Error('Database not ready');
        }

        const { enableCache = false, cacheKey } = options;
        const finalCacheKey = cacheKey || `${sql}-${JSON.stringify(params)}`;

        // Check cache first
        if (enableCache) {
            const cached = queryCache.current.get(finalCacheKey);
            if (cached && isCacheValid(cached)) {
                return cached.data;
            }
        }

        try {
            const stmt = db.prepare(sql);
            
            // Bind parameters if provided
            if (params.length > 0) {
                stmt.bind(params);
            }

            const results: QueryResult[] = [];
            while (stmt.step()) {
                results.push(stmt.getAsObject());
            }
            
            stmt.free();

            // Cache results if enabled
            if (enableCache) {
                queryCache.current.set(finalCacheKey, {
                    data: results,
                    timestamp: Date.now(),
                    ttl: defaultCacheTTL
                });
            }

            return results;
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            throw new Error(`Query failed: ${errorMessage}`);
        }
    }, [db]);

    // Execute non-query SQL (INSERT, UPDATE, DELETE)
    const execute = useCallback(async (sql: string, params: any[] = []): Promise<void> => {
        if (!db) {
            throw new Error('Database not ready');
        }

        try {
            const stmt = db.prepare(sql);
            
            if (params.length > 0) {
                stmt.bind(params);
            }
            
            stmt.step();
            stmt.free();

            // Clear cache after modifications
            queryCache.current.clear();

            // Refresh table list if schema might have changed
            if (sql.toUpperCase().includes('CREATE TABLE') || sql.toUpperCase().includes('DROP TABLE')) {
                const tables = await getTableList(db);
                setState(prev => ({ ...prev, tables }));
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            throw new Error(`Execute failed: ${errorMessage}`);
        }
    }, [db]);

    // Get table schema
    const getTableSchema = useCallback(async (tableName: string): Promise<QueryResult[]> => {
        return query(`PRAGMA table_info(${tableName});`);
    }, [query]);

    // Count records in table
    const getTableCount = useCallback(async (tableName: string): Promise<number> => {
        const result = await query(`SELECT COUNT(*) as count FROM ${tableName};`);
        return result[0]?.count || 0;
    }, [query]);

    // Clear query cache
    const clearCache = useCallback(() => {
        queryCache.current.clear();
    }, []);

    // Export database
    const exportDatabase = useCallback((): Uint8Array | null => {
        if (!db) return null;
        return db.export();
    }, [db]);

    // Load new database
    const loadNewDatabase = useCallback(async (url: string) => {
        if (!SQL) {
            throw new Error('SQL.js not initialized');
        }
        await loadDatabase(SQL, url);
    }, [SQL]);

    return {
        // State
        ...state,
        
        // Core methods
        query,
        execute,
        
        // Utility methods
        getTableSchema,
        getTableCount,
        clearCache,
        exportDatabase,
        loadNewDatabase,
        
        // Direct database access (use with caution)
        database: db
    };
}

// Hook for specific table operations
export function useTable(tableName: string, databaseUrl?: string) {
    const db = useSQLiteDatabase(databaseUrl);

    // Get all records from table
    const getAll = useCallback(async (limit?: number): Promise<QueryResult[]> => {
        const limitClause = limit ? ` LIMIT ${limit}` : '';
        return db.query(`SELECT * FROM ${tableName}${limitClause};`);
    }, [db.query, tableName]);

    // Get record by ID
    const getById = useCallback(async (id: any): Promise<QueryResult | null> => {
        const results = await db.query(`SELECT * FROM ${tableName} WHERE id = ?;`, [id]);
        return results[0] || null;
    }, [db.query, tableName]);

    // Find records with WHERE clause
    const find = useCallback(async (whereClause: string, params: any[] = []): Promise<QueryResult[]> => {
        return db.query(`SELECT * FROM ${tableName} WHERE ${whereClause};`, params);
    }, [db.query, tableName]);

    // Insert record
    const insert = useCallback(async (data: Record<string, any>): Promise<void> => {
        const columns = Object.keys(data).join(', ');
        const placeholders = Object.keys(data).map(() => '?').join(', ');
        const values = Object.values(data);
        
        await db.execute(
            `INSERT INTO ${tableName} (${columns}) VALUES (${placeholders});`,
            values
        );
    }, [db.execute, tableName]);

    // Update record
    const update = useCallback(async (id: any, data: Record<string, any>): Promise<void> => {
        const setClause = Object.keys(data).map(key => `${key} = ?`).join(', ');
        const values = [...Object.values(data), id];
        
        await db.execute(
            `UPDATE ${tableName} SET ${setClause} WHERE id = ?;`,
            values
        );
    }, [db.execute, tableName]);

    // Delete record
    const remove = useCallback(async (id: any): Promise<void> => {
        await db.execute(`DELETE FROM ${tableName} WHERE id = ?;`, [id]);
    }, [db.execute, tableName]);

    return {
        ...db,
        
        // Table-specific methods
        getAll,
        getById,
        find,
        insert,
        update,
        remove,
        
        // Table info
        tableName
    };
}

// Hook for query-based data fetching with React patterns
export function useSQLQuery<T = QueryResult>(
    sql: string, 
    params: any[] = [], 
    options: QueryOptions & { 
        enabled?: boolean;
        databaseUrl?: string;
        databaseName?: string;
    } = {}
) {
    const { enabled = true, databaseUrl, databaseName, ...queryOptions } = options;
    
    // Try to get database from context first, then fall back to direct connection
    const context = useContext(DatabaseContext);
    const directDb = useSQLiteDatabase(databaseUrl);
    
    // Determine which database to use
    const getDatabaseQuery = () => {
        if (databaseName && context?.databases.has(databaseName)) {
            // Use specific named database from context
            const db = context.databases.get(databaseName)!;
            return {
                query: async (sql: string, params: any[], options: QueryOptions) => {
                    const stmt = db.prepare(sql);
                    if (params.length > 0) stmt.bind(params);
                    const results: QueryResult[] = [];
                    while (stmt.step()) {
                        results.push(stmt.getAsObject());
                    }
                    stmt.free();
                    return results;
                },
                isReady: true
            };
        } else if (context?.defaultDatabase && context.databases.has(context.defaultDatabase)) {
            // Use default database from context
            const db = context.databases.get(context.defaultDatabase)!;
            return {
                query: async (sql: string, params: any[], options: QueryOptions) => {
                    const stmt = db.prepare(sql);
                    if (params.length > 0) stmt.bind(params);
                    const results: QueryResult[] = [];
                    while (stmt.step()) {
                        results.push(stmt.getAsObject());
                    }
                    stmt.free();
                    return results;
                },
                isReady: true
            };
        } else {
            // Use direct database connection
            return directDb;
        }
    };

    const { query, isReady } = getDatabaseQuery();
    const [data, setData] = useState<T[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const executeQuery = useCallback(async () => {
        if (!isReady || !enabled) return;

        setIsLoading(true);
        setError(null);

        try {
            const results = await query(sql, params, queryOptions);
            setData(results as T[]);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Unknown error';
            setError(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [query, sql, JSON.stringify(params), isReady, enabled]);

    useEffect(() => {
        executeQuery();
    }, [executeQuery]);

    return {
        data,
        isLoading,
        error,
        refetch: executeQuery,
        // Return which database is being used for debugging
        usingDatabase: databaseName || context?.defaultDatabase || 'direct-connection'
    };
}

// Additional utility hooks
export function useEnvironmentDatabase() {
    const env = process.env.NODE_ENV || 'development';
    const [databaseUrl, setDatabaseUrl] = useState<string>('');

    useEffect(() => {
        const dbMap = {
            'development': '/dev-database.sqlite',
            'test': '/test-database.sqlite',
            'production': '/prod-database.sqlite'
        };
        
        setDatabaseUrl(dbMap[env as keyof typeof dbMap] || '/default-database.sqlite');
    }, [env]);

    return databaseUrl;
}

export function useDynamicDatabase() {
    const [currentDb, setCurrentDb] = useState<string>('/default-database.sqlite');
    
    const switchDatabase = useCallback((newDbUrl: string) => {
        setCurrentDb(newDbUrl);
    }, []);

    return {
        currentDatabase: currentDb,
        switchDatabase,
        // Pre-configured database switchers
        useUserDatabase: () => switchDatabase('/users.sqlite'),
        useProductDatabase: () => switchDatabase('/products.sqlite'),
        useAnalyticsDatabase: () => switchDatabase('/analytics.sqlite')
    };
}