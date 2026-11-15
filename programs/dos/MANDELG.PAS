{ MANDELG.PAS - VGA Mode 13h (320x200x256) Mandelbrot with 80387 + INT 33h mouse zoom }
{$N+,E-}
program MandelG;

uses Dos;

const
  Width = 320;
  Height = 200;

var
  Regs: Registers;
  Cx, Cy, X, Y, XX, YY, NewX: Extended;
  XMin, XMax, YMin, YMax, DX, DY: Extended;
  CenterX, CenterY, SpanX, SpanY: Extended;
  Col, Row, Iter, MaxIter, I, ZoomLevel: Integer;
  Color, R, G, B: Byte;
  HasMouse: Boolean;
  MouseX, MouseY, Buttons: Word;
  Key: Byte;
  StatusStr: string[32];

procedure SetVideoMode(Mode: Word);
begin
  Regs.AX := Mode;
  Intr($10, Regs);
end;

procedure SetPalette;
begin
  Port[$3C8] := 0;
  Port[$3C9] := 0;
  Port[$3C9] := 0;
  Port[$3C9] := 0;
  for I := 1 to 255 do
  begin
    if I < 64 then
    begin
      R := 0;
      G := I div 2;
      B := I;
    end
    else if I < 128 then
    begin
      R := (I - 64);
      G := 31 + (I - 64) div 2;
      B := 63;
    end
    else if I < 192 then
    begin
      R := 63;
      G := 63 - (I - 128) div 2;
      B := 63 - (I - 128);
    end
    else
    begin
      R := 63 - (I - 192);
      G := 31 - (I - 192) div 2;
      B := 0;
    end;
    Port[$3C9] := R;
    Port[$3C9] := G;
    Port[$3C9] := B;
  end;
  Port[$3C8] := 255;
  Port[$3C9] := 63;
  Port[$3C9] := 63;
  Port[$3C9] := 63;
end;

procedure MouseInit;
begin
  Regs.AX := $0000;
  Intr($33, Regs);
  HasMouse := Regs.AX = $FFFF;
end;

procedure MouseShow;
begin
  if HasMouse then
  begin
    Regs.AX := $0001;
    Intr($33, Regs);
  end;
end;

procedure MouseHide;
begin
  if HasMouse then
  begin
    Regs.AX := $0002;
    Intr($33, Regs);
  end;
end;

procedure MouseRead(var MX, MY, Btn: Word);
begin
  if HasMouse then
  begin
    Regs.AX := $0003;
    Intr($33, Regs);
    MX := Regs.CX div 2;
    MY := Regs.DX;
    Btn := Regs.BX;
  end
  else
  begin
    MX := Width div 2;
    MY := Height div 2;
    Btn := 0;
  end;
end;

procedure DrawHudLine(Row: Byte; const S: string);
var
  K: Integer;
begin
  Regs.AH := $02;
  Regs.BH := 0;
  Regs.DH := Row;
  Regs.DL := 0;
  Intr($10, Regs);
  for K := 1 to Length(S) do
  begin
    Regs.AH := $0E;
    Regs.AL := Ord(S[K]);
    Regs.BH := 0;
    Regs.BL := 255;
    Intr($10, Regs);
  end;
end;

function FmtFixed(V: Extended; Decimals: Integer): string;
var
  S: string[20];
  I, D: Integer;
begin
  if V < 0.0 then
  begin
    S := '-';
    V := -V;
  end
  else
    S := '';
  if V >= 10.0 then
  begin
    FmtFixed := S + '>9';
    Exit;
  end;
  D := 0;
  while (D < 9) and (V >= 1.0) do
  begin
    V := V - 1.0;
    Inc(D);
  end;
  S := S + Chr(Ord('0') + D) + '.';
  for I := 1 to Decimals do
  begin
    V := V * 10.0;
    D := 0;
    while (D < 9) and (V >= 1.0) do
    begin
      V := V - 1.0;
      Inc(D);
    end;
    S := S + Chr(Ord('0') + D);
  end;
  FmtFixed := S;
end;

procedure DrawHud;
var
  S1, S2: string[12];
begin
  Str(ZoomLevel, S1);
  Str(MaxIter, S2);
  DrawHudLine(0, 'Z:' + S1 + ' MaxIter:' + S2 + ' W:' + FmtFixed(SpanX, 4));
  DrawHudLine(1, 'C:(' + FmtFixed(CenterX, 5) + ',' + FmtFixed(CenterY, 5) + ')');
end;

procedure Render;
begin
  MouseHide;
  XMin := CenterX - SpanX * 0.5;
  XMax := CenterX + SpanX * 0.5;
  YMin := CenterY - SpanY * 0.5;
  YMax := CenterY + SpanY * 0.5;
  DX := (XMax - XMin) / Width;
  DY := (YMax - YMin) / Height;

  for Row := 0 to Height - 1 do
  begin
    Cy := YMin + Row * DY;
    for Col := 0 to Width - 1 do
    begin
      Cx := XMin + Col * DX;
      X := 0.0;
      Y := 0.0;
      Iter := 0;
      while Iter < MaxIter do
      begin
        XX := X * X;
        YY := Y * Y;
        if XX + YY > 4.0 then
          Break;
        NewX := XX - YY + Cx;
        Y := 2.0 * X * Y + Cy;
        X := NewX;
        Inc(Iter);
      end;
      if Iter = MaxIter then
        Color := 0
      else
        Color := 1 + (((Iter * 12) + 24) mod 254);
      Mem[$A000:Row * Width + Col] := Color;
    end;
  end;

  DrawHud;
  MouseShow;
end;

procedure ResetView;
begin
  CenterX := -0.6;
  CenterY := 0.0;
  SpanX := 3.2;
  SpanY := 2.4;
  ZoomLevel := 0;
  MaxIter := 32;
end;

begin
  ResetView;
  SetVideoMode($0013);
  SetPalette;
  MouseInit;
  Render;

  repeat
    MouseRead(MouseX, MouseY, Buttons);
    if (Buttons and 1) <> 0 then
    begin
      CenterX := XMin + MouseX * DX;
      CenterY := YMin + MouseY * DY;
      SpanX := SpanX * 0.5;
      SpanY := SpanY * 0.5;
      Inc(ZoomLevel);
      MaxIter := 32 + ZoomLevel * 24;
      if MaxIter > 512 then MaxIter := 512;
      repeat
        MouseRead(MouseX, MouseY, Buttons);
      until (Buttons and 1) = 0;
      Render;
    end
    else if (Buttons and 2) <> 0 then
    begin
      if ZoomLevel > 0 then
      begin
        CenterX := XMin + MouseX * DX;
        CenterY := YMin + MouseY * DY;
        SpanX := SpanX * 2.0;
        SpanY := SpanY * 2.0;
        Dec(ZoomLevel);
        MaxIter := 32 + ZoomLevel * 24;
      end;
      repeat
        MouseRead(MouseX, MouseY, Buttons);
      until (Buttons and 2) = 0;
      Render;
    end;

    Regs.AH := $01;
    Intr($16, Regs);
    if (Regs.Flags and FZero) = 0 then
    begin
      Regs.AH := $00;
      Intr($16, Regs);
      Key := Regs.AL;
      if (Key = 27) or (Key = Ord('q')) or (Key = Ord('Q')) then
        Break
      else if (Key = Ord('r')) or (Key = Ord('R')) then
      begin
        ResetView;
        Render;
      end;
    end;
  until False;

  MouseHide;
  SetVideoMode($0003);
end.
