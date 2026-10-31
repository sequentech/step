// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A small GraphQL executor for the stand-in: it parses the portals'
// operations (variables, fragments, aliases, @include/@skip), resolves root
// fields with the stand-in's resolvers or its in-memory tables, and shapes
// every answer by the portals' checked-in graphql.schema.json, so that
// `__typename` and introspection answer as Hasura would.

// -- Lexer --------------------------------------------------------------------

const PUNCTUATORS = new Set(["!", "$", "&", "(", ")", "[", "]", "{", "}", ":", "=", "@", "|"]);

function lex(source) {
  const tokens = [];
  let i = 0;
  while (i < source.length) {
    const c = source[i];
    if (c === "#") {
      while (i < source.length && source[i] !== "\n") i++;
    } else if (/[\s,\ufeff]/.test(c)) {
      i++;
    } else if (source.startsWith("...", i)) {
      tokens.push({ k: "p", v: "..." });
      i += 3;
    } else if (PUNCTUATORS.has(c)) {
      tokens.push({ k: "p", v: c });
      i++;
    } else if (/[A-Za-z_]/.test(c)) {
      const m = /^[A-Za-z_][A-Za-z0-9_]*/.exec(source.slice(i, i + 256));
      tokens.push({ k: "n", v: m[0] });
      i += m[0].length;
    } else if (/[-0-9]/.test(c)) {
      const m = /^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/.exec(source.slice(i, i + 64));
      if (!m) throw new SyntaxError(`unexpected ${c}`);
      tokens.push({ k: m[2] || m[3] ? "f" : "i", v: m[0] });
      i += m[0].length;
    } else if (source.startsWith('"""', i)) {
      const end = source.indexOf('"""', i + 3);
      if (end < 0) throw new SyntaxError("unterminated block string");
      tokens.push({ k: "s", v: source.slice(i + 3, end).replace(/\\"""/g, '"""') });
      i = end + 3;
    } else if (c === '"') {
      let j = i + 1;
      let out = "";
      while (source[j] !== '"') {
        if (j >= source.length || source[j] === "\n") throw new SyntaxError("unterminated string");
        if (source[j] === "\\") {
          const e = source[j + 1];
          if (e === "u") {
            out += String.fromCharCode(parseInt(source.slice(j + 2, j + 6), 16));
            j += 6;
            continue;
          }
          out += { b: "\b", f: "\f", n: "\n", r: "\r", t: "\t" }[e] ?? e;
          j += 2;
        } else {
          out += source[j++];
        }
      }
      tokens.push({ k: "s", v: out });
      i = j + 1;
    } else {
      throw new SyntaxError(`unexpected ${JSON.stringify(c)}`);
    }
  }
  return tokens;
}

// -- Parser -------------------------------------------------------------------

export function parse(source) {
  const t = lex(source);
  let p = 0;
  const peek = (v) => t[p] && (v === undefined || t[p].v === v) && t[p];
  const expect = (v) => {
    if (!peek(v)) throw new SyntaxError(`expected ${v} at token ${p}`);
    return t[p++];
  };
  const name = () => {
    if (!t[p] || t[p].k !== "n") throw new SyntaxError(`expected a name at token ${p}`);
    return t[p++].v;
  };

  function value(constant) {
    const tok = t[p];
    if (!tok) throw new SyntaxError("expected a value");
    if (tok.v === "$" && tok.k === "p" && !constant) {
      p++;
      return { var: name() };
    }
    if (tok.v === "[" && tok.k === "p") {
      p++;
      const list = [];
      while (!peek("]")) list.push(value(constant));
      p++;
      return { list };
    }
    if (tok.v === "{" && tok.k === "p") {
      p++;
      const fields = [];
      while (!peek("}")) {
        const key = name();
        expect(":");
        fields.push([key, value(constant)]);
      }
      p++;
      return { object: fields };
    }
    p++;
    if (tok.k === "i") return { const: Number(tok.v) };
    if (tok.k === "f") return { const: Number(tok.v) };
    if (tok.k === "s") return { const: tok.v };
    if (tok.k === "n") {
      if (tok.v === "true") return { const: true };
      if (tok.v === "false") return { const: false };
      if (tok.v === "null") return { const: null };
      return { const: tok.v };
    }
    throw new SyntaxError(`unexpected ${tok.v}`);
  }

  function args() {
    const out = [];
    if (!peek("(")) return out;
    p++;
    while (!peek(")")) {
      const key = name();
      expect(":");
      out.push([key, value(false)]);
    }
    p++;
    return out;
  }

  function directives() {
    const out = [];
    while (peek("@")) {
      p++;
      out.push({ name: name(), args: args() });
    }
    return out;
  }

  function type() {
    let ty;
    if (peek("[")) {
      p++;
      ty = { list: type() };
      expect("]");
    } else {
      ty = { name: name() };
    }
    if (peek("!")) {
      p++;
      ty = { nonNull: ty };
    }
    return ty;
  }

  function selectionSet() {
    expect("{");
    const out = [];
    while (!peek("}")) {
      if (peek("...")) {
        p++;
        if (t[p]?.k === "n" && t[p].v !== "on") {
          out.push({ spread: name(), directives: directives() });
        } else {
          let on;
          if (peek("on")) {
            p++;
            on = name();
          }
          out.push({ inline: true, on, directives: directives(), selections: selectionSet() });
        }
      } else {
        let alias;
        let fieldName = name();
        if (peek(":")) {
          p++;
          alias = fieldName;
          fieldName = name();
        }
        const field = { name: fieldName, alias, args: args(), directives: directives() };
        if (peek("{")) field.selections = selectionSet();
        out.push(field);
      }
    }
    p++;
    return out;
  }

  const operations = [];
  const fragments = new Map();
  while (p < t.length) {
    if (peek("{")) {
      operations.push({ kind: "query", variables: [], selections: selectionSet() });
    } else if (peek("fragment")) {
      p++;
      const fragmentName = name();
      expect("on");
      const on = name();
      directives();
      fragments.set(fragmentName, { on, selections: selectionSet() });
    } else {
      const kind = name();
      if (!["query", "mutation", "subscription"].includes(kind)) throw new SyntaxError(`unexpected ${kind}`);
      const op = { kind, variables: [] };
      if (t[p]?.k === "n") op.name = name();
      if (peek("(")) {
        p++;
        while (!peek(")")) {
          expect("$");
          const variable = name();
          expect(":");
          const ty = type();
          let defaultValue;
          if (peek("=")) {
            p++;
            defaultValue = value(true);
          }
          directives();
          op.variables.push({ name: variable, type: ty, defaultValue });
        }
        p++;
      }
      directives();
      op.selections = selectionSet();
      operations.push(op);
    }
  }
  return { operations, fragments };
}

// -- Schema --------------------------------------------------------------------

export class Schema {
  constructor(json) {
    this.json = json.__schema ?? json.data?.__schema;
    this.types = new Map();
    for (const ty of this.json.types) {
      const fields = new Map();
      for (const f of ty.fields ?? []) fields.set(f.name, f);
      this.types.set(ty.name, { ...ty, fieldMap: fields });
    }
    this.query = this.json.queryType?.name;
    this.mutation = this.json.mutationType?.name;
  }

  field(typeName, fieldName) {
    return this.types.get(typeName)?.fieldMap.get(fieldName);
  }
}

const named = (ref) => {
  while (ref && ref.ofType) ref = ref.ofType;
  return ref?.name;
};

// -- Execution -------------------------------------------------------------------

export class GraphQLError extends Error {
  constructor(message, code = "unexpected", extra = {}) {
    super(message);
    this.code = code;
    this.extra = extra;
  }
}

function argValue(node, variables) {
  if (node.var !== undefined) return variables[node.var];
  if (node.list) return node.list.map((v) => argValue(v, variables));
  if (node.object) {
    const o = {};
    for (const [k, v] of node.object) {
      const x = argValue(v, variables);
      if (x !== undefined) o[k] = x;
    }
    return o;
  }
  return node.const;
}

function argsOf(field, variables) {
  const o = {};
  for (const [k, v] of field.args) {
    const x = argValue(v, variables);
    if (x !== undefined) o[k] = x;
  }
  return o;
}

function included(node, variables) {
  for (const d of node.directives ?? []) {
    const a = argsOf(d, variables);
    if (d.name === "skip" && a.if === true) return false;
    if (d.name === "include" && a.if === false) return false;
  }
  return true;
}

/**
 * `resolvers.root(kind, fieldName, args, ctx, info)` answers a root field
 * (returning `undefined` when it doesn't know it); `resolvers.field(typeName,
 * fieldName, parent, args, ctx)` may answer a nested field.
 */
export class Executor {
  constructor(schema, resolvers, log = () => {}) {
    this.schema = schema;
    this.resolvers = resolvers;
    this.log = log;
  }

  collect(typeName, selections, variables, fragments, out = []) {
    for (const s of selections) {
      if (!included(s, variables)) continue;
      if (s.spread !== undefined) {
        const f = fragments.get(s.spread);
        if (!f) throw new GraphQLError(`fragment ${s.spread} isn't defined`, "validation-failed");
        if (this.applies(f.on, typeName)) this.collect(typeName, f.selections, variables, fragments, out);
      } else if (s.inline) {
        if (!s.on || this.applies(s.on, typeName)) this.collect(typeName, s.selections, variables, fragments, out);
      } else {
        out.push(s);
      }
    }
    return out;
  }

  applies(condition, typeName) {
    if (condition === typeName) return true;
    const ty = this.schema.types.get(condition);
    return Boolean(ty && (ty.kind === "INTERFACE" || ty.kind === "UNION") &&
      (ty.possibleTypes ?? []).some((pt) => pt.name === typeName));
  }

  async complete(ref, value, field, ctx) {
    if (ref?.kind === "NON_NULL") return this.complete(ref.ofType, value, field, ctx);
    if (value === undefined || value === null) return null;
    if (ref?.kind === "LIST") {
      const list = Array.isArray(value) ? value : [value];
      return Promise.all(list.map((v) => this.complete(ref.ofType, v, field, ctx)));
    }
    const typeName = ref?.name;
    const ty = typeName && this.schema.types.get(typeName);
    if (!field.selections || ty?.kind === "SCALAR" || ty?.kind === "ENUM") return value;
    if (!ty) return this.shape(value, field, ctx);
    return this.object(typeName, value, field.selections, ctx);
  }

  /** A value of a type the schema doesn't know (an action the checked-in
   * schema predates), cut to the selection set. */
  shape(value, field, ctx) {
    if (value === null || value === undefined || typeof value !== "object" || !field.selections) return value ?? null;
    if (Array.isArray(value)) return value.map((v) => this.shape(v, field, ctx));
    const out = {};
    for (const f of this.collect(undefined, field.selections, ctx.variables, ctx.fragments)) {
      out[f.alias ?? f.name] = f.name === "__typename" ? "Object" : this.shape(value[f.name] ?? null, f, ctx);
    }
    return out;
  }

  async object(typeName, value, selections, ctx) {
    const out = {};
    for (const f of this.collect(typeName, selections, ctx.variables, ctx.fragments)) {
      const key = f.alias ?? f.name;
      if (f.name === "__typename") {
        out[key] = typeName;
        continue;
      }
      const def = this.schema.field(typeName, f.name);
      const args = argsOf(f, ctx.variables);
      let v = await this.resolvers.field?.(typeName, f.name, value, args, ctx);
      if (v === undefined) v = value[f.name];
      if (v === undefined) {
        if (!def) this.log(`no field ${typeName}.${f.name}`);
        v = defaultFor(def?.type);
      }
      out[key] = await this.complete(def?.type ?? guessRef(v), v, f, ctx);
    }
    return out;
  }

  async execute({ query, variables = {}, operationName }, ctx = {}) {
    let doc;
    try {
      doc = parse(query);
    } catch (e) {
      return { errors: [hasuraError(new GraphQLError(`not a valid graphql query: ${e.message}`, "validation-failed"))] };
    }
    const op = operationName
      ? doc.operations.find((o) => o.name === operationName)
      : doc.operations.length === 1 ? doc.operations[0] : undefined;
    if (!op) return { errors: [hasuraError(new GraphQLError("operationName is required", "validation-failed"))] };
    const vars = { ...(variables ?? {}) };
    for (const v of op.variables) {
      if (vars[v.name] === undefined && v.defaultValue) vars[v.name] = argValue(v.defaultValue, {});
    }
    const rootType = op.kind === "mutation" ? this.schema.mutation : this.schema.query;
    const exec = { ...ctx, variables: vars, fragments: doc.fragments, operation: op };
    const data = {};
    try {
      for (const f of this.collect(rootType, op.selections, vars, doc.fragments)) {
        const key = f.alias ?? f.name;
        if (f.name === "__typename") {
          data[key] = rootType;
          continue;
        }
        const args = argsOf(f, vars);
        if (f.name === "__schema") {
          data[key] = await this.object("__Schema", this.schema.json, f.selections, exec);
          continue;
        }
        if (f.name === "__type") {
          const ty = this.schema.json.types.find((x) => x.name === args.name);
          data[key] = ty ? await this.object("__Type", ty, f.selections, exec) : null;
          continue;
        }
        const def = this.schema.field(rootType, f.name);
        let v = await this.resolvers.root(op.kind, f.name, args, exec, { field: f, def, returns: def && named(def.type) });
        if (v === undefined) {
          if (!def) throw new GraphQLError(`field '${f.name}' not found in type: '${rootType}'`, "validation-failed");
          this.log(`no resolver for ${op.kind} ${f.name}`);
          v = defaultFor(def.type);
        }
        data[key] = def ? await this.complete(def.type, v, f, exec) : this.shape(v, f, exec);
      }
    } catch (e) {
      if (!(e instanceof GraphQLError)) this.log(`error: ${e.stack ?? e}`);
      return { errors: [hasuraError(e instanceof GraphQLError ? e : new GraphQLError(String(e.message ?? e)))] };
    }
    return { data };
  }
}

function defaultFor(ref) {
  if (!ref) return null;
  if (ref.kind === "NON_NULL") {
    const inner = ref.ofType;
    if (inner.kind === "LIST") return [];
    if (inner.name === "Boolean") return false;
    if (inner.name === "Int" || inner.name === "bigint" || inner.name === "Float") return 0;
    const n = named(inner);
    if (n && n.endsWith("_aggregate")) return { aggregate: { count: 0 }, nodes: [] };
    if (n && n.endsWith("_aggregate_fields")) return { count: 0 };
    return null;
  }
  return null;
}

const guessRef = () => undefined;

/** An error as Hasura answers it. */
export function hasuraError(e) {
  return { message: e.message, extensions: { path: "$", code: e.code ?? "unexpected", ...(e.extra ?? {}) } };
}

// -- Tables -----------------------------------------------------------------------

const OPERATORS = new Set([
  "_eq", "_neq", "_in", "_nin", "_is_null", "_gt", "_gte", "_lt", "_lte", "_like", "_ilike",
  "_nlike", "_nilike", "_similar", "_contains", "_contained_in", "_has_key", "_has_keys_any", "_has_keys_all",
]);

const same = (a, b) => (a === b) || (a != null && b != null && String(a) === String(b));

const likeRegex = (pattern, flags) =>
  new RegExp(`^${pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/%/g, ".*").replace(/_/g, ".")}$`, flags);

function contains(haystack, needle) {
  if (needle === null || typeof needle !== "object") return same(haystack, needle);
  if (haystack === null || typeof haystack !== "object") return false;
  if (Array.isArray(needle)) return needle.every((n) => (haystack ?? []).some((h) => contains(h, n)));
  return Object.entries(needle).every(([k, v]) => contains(haystack[k], v));
}

function test(value, conditions) {
  if (conditions === null || typeof conditions !== "object") return true;
  for (const [op, x] of Object.entries(conditions)) {
    if (!OPERATORS.has(op)) {
      // A filter on a relationship the stand-in doesn't hold.
      if (value && typeof value === "object") {
        const rows = Array.isArray(value) ? value : [value];
        if (!rows.some((r) => matches(r, { [op]: x }))) return false;
      }
      continue;
    }
    if (x === null && op !== "_is_null") continue;
    switch (op) {
      case "_eq": if (!same(value, x)) return false; break;
      case "_neq": if (same(value, x)) return false; break;
      case "_in": if (!x.some((y) => same(value, y))) return false; break;
      case "_nin": if (x.some((y) => same(value, y))) return false; break;
      case "_is_null": if ((value === null || value === undefined) !== Boolean(x)) return false; break;
      case "_gt": if (!(value > x)) return false; break;
      case "_gte": if (!(value >= x)) return false; break;
      case "_lt": if (!(value < x)) return false; break;
      case "_lte": if (!(value <= x)) return false; break;
      case "_like": if (!likeRegex(x, "").test(String(value ?? ""))) return false; break;
      case "_ilike": if (!likeRegex(x, "i").test(String(value ?? ""))) return false; break;
      case "_nlike": if (likeRegex(x, "").test(String(value ?? ""))) return false; break;
      case "_nilike": if (likeRegex(x, "i").test(String(value ?? ""))) return false; break;
      case "_contains": if (!contains(value, x)) return false; break;
      case "_contained_in": if (!contains(x, value)) return false; break;
      case "_has_key": if (!(value && typeof value === "object" && x in value)) return false; break;
      case "_has_keys_any": if (!(value && x.some((k) => k in value))) return false; break;
      case "_has_keys_all": if (!(value && x.every((k) => k in value))) return false; break;
      default: break;
    }
  }
  return true;
}

export function matches(row, where) {
  if (!where || typeof where !== "object") return true;
  for (const [k, cond] of Object.entries(where)) {
    if (k === "_and") {
      if (!(Array.isArray(cond) ? cond : [cond]).every((c) => matches(row, c))) return false;
    } else if (k === "_or") {
      const list = Array.isArray(cond) ? cond : [cond];
      if (list.length > 0 && !list.some((c) => matches(row, c))) return false;
    } else if (k === "_not") {
      if (matches(row, cond)) return false;
    } else if (!test(row[k], cond)) {
      return false;
    }
  }
  return true;
}

function sortRows(rows, orderBy) {
  const keys = (Array.isArray(orderBy) ? orderBy : orderBy ? [orderBy] : [])
    .flatMap((o) => Object.entries(o))
    .filter(([, dir]) => typeof dir === "string");
  if (keys.length === 0) return rows;
  return [...rows].sort((a, b) => {
    for (const [k, dir] of keys) {
      const x = a[k];
      const y = b[k];
      if (same(x, y)) continue;
      const desc = dir.startsWith("desc");
      if (x === null || x === undefined) return dir.includes("nulls_first") ? -1 : 1;
      if (y === null || y === undefined) return dir.includes("nulls_first") ? 1 : -1;
      return (x < y ? -1 : 1) * (desc ? -1 : 1);
    }
    return 0;
  });
}

/** Hasura's generated root fields over in-memory tables. */
export class Tables {
  constructor() {
    this.rows = new Map();
  }

  table(name) {
    if (!this.rows.has(name)) this.rows.set(name, []);
    return this.rows.get(name);
  }

  select(name, { where, order_by: orderBy, offset = 0, limit } = {}) {
    const rows = sortRows(this.table(name).filter((r) => matches(r, where)), orderBy);
    return rows.slice(offset, limit === undefined || limit === null ? undefined : offset + limit);
  }

  /** Answers a generated root field, or undefined if `field` isn't one. */
  root(kind, field, args) {
    let m;
    if (kind !== "mutation") {
      if ((m = /^(sequent_backend_\w+?)_aggregate$/.exec(field))) {
        const nodes = this.select(m[1], args);
        return { aggregate: { count: nodes.length }, nodes };
      }
      if ((m = /^(sequent_backend_\w+?)_by_pk$/.exec(field))) {
        return this.table(m[1]).find((r) => Object.entries(args).every(([k, v]) => same(r[k], v))) ?? null;
      }
      if (/^sequent_backend_\w+$/.test(field)) return this.select(field, args);
      return undefined;
    }
    if ((m = /^insert_(sequent_backend_\w+?)_one$/.exec(field))) return this.insert(m[1], [args.object])[0];
    if ((m = /^insert_(sequent_backend_\w+)$/.exec(field))) {
      const returning = this.insert(m[1], args.objects ?? []);
      return { affected_rows: returning.length, returning };
    }
    if ((m = /^update_(sequent_backend_\w+?)_by_pk$/.exec(field))) {
      const where = Object.fromEntries(Object.entries(args.pk_columns ?? {}).map(([k, v]) => [k, { _eq: v }]));
      return this.update(m[1], where, args._set)[0] ?? null;
    }
    if ((m = /^update_(sequent_backend_\w+)$/.exec(field))) {
      const returning = this.update(m[1], args.where, args._set);
      return { affected_rows: returning.length, returning };
    }
    if ((m = /^delete_(sequent_backend_\w+?)_by_pk$/.exec(field))) {
      const where = Object.fromEntries(Object.entries(args).map(([k, v]) => [k, { _eq: v }]));
      return this.remove(m[1], where)[0] ?? null;
    }
    if ((m = /^delete_(sequent_backend_\w+)$/.exec(field))) {
      const returning = this.remove(m[1], args.where);
      return { affected_rows: returning.length, returning };
    }
    return undefined;
  }

  insert(name, objects) {
    const now = new Date().toISOString();
    const rows = objects.map((o) => ({ id: o.id ?? crypto.randomUUID(), created_at: now, last_updated_at: now, ...o }));
    this.table(name).push(...rows);
    return rows;
  }

  update(name, where, set = {}) {
    const rows = this.table(name).filter((r) => matches(r, where));
    for (const r of rows) Object.assign(r, set, { last_updated_at: new Date().toISOString() });
    return rows;
  }

  remove(name, where) {
    const all = this.table(name);
    const gone = all.filter((r) => matches(r, where));
    this.rows.set(name, all.filter((r) => !gone.includes(r)));
    return gone;
  }
}
