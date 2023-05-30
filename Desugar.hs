module Desugar(desugar) where
import Parse
import Exp

type LDef = (Ident, Exp)

desugar :: Module -> [LDef]
desugar (Module _ ds) = concatMap dsDef ds

lams :: [Ident] -> Exp -> Exp
lams xs e = foldr Lam e xs

apps :: Exp -> [Exp] -> Exp
apps f xs = foldl App f xs

dsDef :: Def -> [LDef]
dsDef (Data _ cs) = zipWith dsConstr [0..] cs
  where
    fs = [f i | (i, _) <- zip [0..] cs]
    dsConstr i (c, ts) = (c, lams xs $ lams fs $ apps (Var (f i)) (map Var xs))
      where xs = ["$x" ++ show j | (j, _) <- zip [0..] ts]
    f i = "$f" ++ show i
dsDef (Fcn (f, xs) e) = [(f, lams xs $ dsExpr e)]
dsDef (Sign _ _) = []

dsExpr :: Expr -> Exp
dsExpr (EVar i) = Var i
dsExpr (EApp f a) = App (dsExpr f) (dsExpr a)
dsExpr (ELam x e) = Lam x (dsExpr e)
dsExpr (EInt i) = Int i
dsExpr (ECase e as) = apps (dsExpr e) (map dsArm as)
  where dsArm (PConstr _ vs, r) = lams vs $ dsExpr r
        dsArm (PTuple [_], _) = error "dsExpr: singleton tuple"
        dsArm (PTuple vs, r) = lams vs $ dsExpr r
-- For now, just sequential bindings.
dsExpr (ELet ds e) =
  let ds' = concatMap dsDef ds
      e' = dsExpr e
      def (i, d) e = App (Lam i e) d
  in  foldr def e' ds'
dsExpr (EList es) =
  foldr (App2 CO) CK $ map dsExpr es
dsExpr (ETuple []) = Lam "_x" (Var "_x")    -- encoding of ()
dsExpr (ETuple [e]) = dsExpr e
dsExpr (ETuple es) = Lam "_f" $ foldl App (Var "_f") $ map dsExpr es
