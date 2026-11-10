{ MANDELFP.PAS -- Turbo Pascal 6/7 Mandelbrot regression benchmark. }
{ Software Real by default. For future x87 Extended testing, change }
{ the following line to: {$DEFINE USE387} }
{ $DEFINE USE387 }

{$IFDEF USE387}
  {$N+}
  {$E-}
{$ELSE}
  {$N-}
{$ENDIF}

program MandelFP;

const
  Size = 16;
  MaxIter = 32;

type
{$IFDEF USE387}
  TNumber = Extended;
{$ELSE}
  TNumber = Real;
{$ENDIF}

var
  Cx, Cy, X, Y, XX, YY, NewX, Step: TNumber;
  Row, Col, Iter, Value: Integer;
  Sum, Weighted: LongInt;
  Escaped: Boolean;

begin
{$IFDEF USE387}
  WriteLn('MANDFP: 80387 Extended');
{$ELSE}
  WriteLn('MANDFP: Software Real');
{$ENDIF}
  Step := 3.0 / Size;
  Sum := 0;
  Weighted := 0;
  for Row := 0 to Size - 1 do
  begin
    Cy := -1.5 + Row * Step;
    for Col := 0 to Size - 1 do
    begin
      Cx := -2.0 + Col * Step;
      X := 0.0;
      Y := 0.0;
      Iter := 0;
      Escaped := False;
      while (Iter < MaxIter) and not Escaped do
      begin
        XX := X * X;
        YY := Y * Y;
        if XX + YY > 4.0 then
          Escaped := True
        else
        begin
          NewX := XX - YY + Cx;
          Y := 2.0 * X * Y + Cy;
          X := NewX;
          Inc(Iter);
        end;
      end;
      if Escaped then
        Value := Iter
      else
        Value := 0;
      Sum := Sum + Value;
      Weighted := Weighted +
                  Value * (Row * Size + Col + 1);
      if not Escaped then
        Write('#')
      else if Iter < 10 then
        Write(Chr(Ord('0') + Iter))
      else
        Write(Chr(Ord('A') + Iter - 10));
    end;
    WriteLn;
  end;
  WriteLn('SUM=', Sum);
  WriteLn('WEIGHTED=', Weighted);
end.
