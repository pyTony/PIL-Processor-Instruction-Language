{ MANDELG.PAS - VGA Mode 13h (320x200x256) Mandelbrot with 80387 + INT 33h mouse zoom }
{$N+,E-}
program MandelG;

uses Dos;

const
  Width = 320;
  Height = 200;
  ActNone    = 0;
  ActZoomIn  = 1;
  ActZoomOut = 2;
  ActReset   = 3;
  ActQuit    = 4;

type
  TFrame = array[0..Height - 1, 0..Width - 1] of Byte;
  PFrame = ^TFrame;

var
  Regs: Registers;
  Cx, Cy, X, Y, XX, YY, NewX: Extended;
  XMin, XMax, YMin, YMax, DX, DY: Extended;
  CenterX, CenterY, SpanX, SpanY: Extended;
  Col, Row, I, ZoomLevel: Integer;
  Iter, MaxIter: LongInt;
  Color, R, G, B: Byte;
  HasMouse: Boolean;
  FrameCs: LongInt;
  FrameBuf, TempBuf: PFrame;
  PendingAct, Act: Byte;
  PendingX, PendingY: Word;
  HasBox: Boolean;
  BoxX0, BoxX1, BoxY0, BoxY1: Integer;

{ Iteration limit doubles with every zoom level so deep, nearly self-similar
  regions keep resolving. LongInt: the colour formula needs Iter * 12. }
function IterFor(Z: Integer): LongInt;
var
  N: LongInt;
begin
  N := 32;
  while (Z > 0) and (N < 50000) do
  begin
    N := N * 2;
    Dec(Z);
  end;
  if N > 50000 then N := 50000;
  IterFor := N;
end;

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

procedure ClearMousePresses;
begin
  if HasMouse then
  begin
    Regs.AX := $0005;
    Regs.BX := 0;
    Intr($33, Regs);
    Regs.AX := $0005;
    Regs.BX := 1;
    Intr($33, Regs);
  end;
end;

function PollInput(var PAct: Byte; var ClickX, ClickY: Word): Boolean;
var
  Key: Byte;
  MX, MY: Word;
begin
  if HasMouse then
  begin
    Regs.AX := $0005;
    Regs.BX := 0;
    Intr($33, Regs);
    if Regs.BX > 0 then
    begin
      MX := Regs.CX div 2;
      MY := Regs.DX;
      if MX >= Width then MX := Width - 1;
      if MY >= Height then MY := Height - 1;
      PAct := ActZoomIn;
      ClickX := MX;
      ClickY := MY;
      PollInput := True;
      Exit;
    end;
    Regs.AX := $0005;
    Regs.BX := 1;
    Intr($33, Regs);
    if (Regs.BX > 0) and (SpanX < 25.0) then
    begin
      MX := Regs.CX div 2;
      MY := Regs.DX;
      if MX >= Width then MX := Width - 1;
      if MY >= Height then MY := Height - 1;
      PAct := ActZoomOut;
      ClickX := MX;
      ClickY := MY;
      PollInput := True;
      Exit;
    end;
  end;

  Regs.AH := $01;
  Intr($16, Regs);
  if (Regs.Flags and FZero) = 0 then
  begin
    Regs.AH := $00;
    Intr($16, Regs);
    Key := Regs.AL;
    if (Key = 27) or (Key = Ord('q')) or (Key = Ord('Q')) then
    begin
      PAct := ActQuit;
      PollInput := True;
      Exit;
    end
    else if (Key = Ord('r')) or (Key = Ord('R')) then
    begin
      PAct := ActReset;
      PollInput := True;
      Exit;
    end;
  end;

  PAct := ActNone;
  PollInput := False;
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

function NowCs: LongInt;
var
  H, M, S, S100: Word;
begin
  GetTime(H, M, S, S100);
  NowCs := ((LongInt(H) * 60 + LongInt(M)) * 60 + LongInt(S)) * 100 + LongInt(S100);
end;

function FmtTimeCs(Cs: LongInt): string;
var
  SecStr, CStr: string[12];
begin
  if Cs < 0 then
  begin
    FmtTimeCs := '...';
    Exit;
  end;
  Str(Cs div 100, SecStr);
  Str(Cs mod 100, CStr);
  if Length(CStr) < 2 then
    CStr := '0' + CStr;
  FmtTimeCs := SecStr + '.' + CStr + 's';
end;

{ Zoom out shrinks the old frame 3x: a new pixel stands for the 3x3 block of
  old pixels around (SX, SY). Take the most frequent colour of the nine
  (centre wins ties) instead of recalculating it. }
function BlockColor(SY, SX: Integer): Byte;
var
  Cells: array[0..8] of Byte;
  N, K, J, Cnt, BestCnt, DY, DX, PY, PX: Integer;
  Best: Byte;
begin
  N := 0;
  for K := 0 to 8 do
  begin
    if K = 0 then begin DY := 0; DX := 0; end
    else
    begin
      DY := ((K - 1) div 3) - 1;
      DX := ((K - 1) mod 3) - 1;
      if (DY = 0) and (DX = 0) then
      begin
        DY := 1;
        DX := 1;
      end;
    end;
    PY := SY + DY;
    PX := SX + DX;
    if (PY >= 0) and (PY < Height) and (PX >= 0) and (PX < Width) then
    begin
      Cells[N] := FrameBuf^[PY, PX];
      Inc(N);
    end;
  end;
  Best := Cells[0];
  BestCnt := 0;
  for K := 0 to N - 1 do
  begin
    Cnt := 0;
    for J := 0 to N - 1 do
      if Cells[J] = Cells[K] then Inc(Cnt);
    if Cnt > BestCnt then
    begin
      BestCnt := Cnt;
      Best := Cells[K];
    end;
  end;
  BlockColor := Best;
end;

procedure PreviewZoom(ZoomIn: Boolean; ClickX, ClickY: Integer);
var
  RIdx, CIdx, SX, SY: Integer;
begin
  HasBox := False;
  BoxX0 := Width;
  BoxX1 := -1;
  BoxY0 := Height;
  BoxY1 := -1;
  if (FrameBuf = nil) or (TempBuf = nil) then
    Exit;
  for RIdx := 0 to Height - 1 do
  begin
    if ZoomIn then
      SY := ClickY - 33 + (RIdx div 3)
    else
      SY := ClickY + (RIdx - 100) * 3;
    if (SY < 0) or (SY >= Height) then
      FillChar(TempBuf^[RIdx], Width, 0)
    else
    begin
      for CIdx := 0 to Width - 1 do
      begin
        if ZoomIn then
          SX := ClickX - 53 + (CIdx div 3)
        else
          SX := ClickX + (CIdx - 160) * 3;
        if (SX >= 0) and (SX < Width) then
        begin
          if ZoomIn then
            TempBuf^[RIdx, CIdx] := FrameBuf^[SY, SX]
          else
            TempBuf^[RIdx, CIdx] := BlockColor(SY, SX);
          if not ZoomIn then
          begin
            if not HasBox then
            begin
              HasBox := True;
              BoxX0 := CIdx;
              BoxX1 := CIdx;
              BoxY0 := RIdx;
              BoxY1 := RIdx;
            end
            else
            begin
              if CIdx < BoxX0 then BoxX0 := CIdx;
              if CIdx > BoxX1 then BoxX1 := CIdx;
              if RIdx < BoxY0 then BoxY0 := RIdx;
              if RIdx > BoxY1 then BoxY1 := RIdx;
            end;
          end;
        end
        else
          TempBuf^[RIdx, CIdx] := 0;
      end;
    end;
  end;
  Move(TempBuf^, FrameBuf^, SizeOf(TFrame));
  MouseHide;
  Move(FrameBuf^[9, 0], Mem[$A000:9 * Width], (Height - 9) * Width);
  MouseShow;
end;

procedure DrawHud;
var
  S1, S2: string[12];
begin
  MouseHide;
  FillChar(Mem[$A000:0], 9 * Width, 0);
  Str(ZoomLevel, S1);
  Str(MaxIter, S2);
  DrawHudLine(0, 'Z' + S1 + ' I' + S2 + ' T:' + FmtTimeCs(FrameCs) +
                 ' (' + FmtFixed(CenterX, 3) + ',' + FmtFixed(CenterY, 3) + ')');
  MouseShow;
end;

function CalcColor(PxCx, PxCy: Extended): Byte;
begin
  X := 0.0;
  Y := 0.0;
  Iter := 0;
  while Iter < MaxIter do
  begin
    XX := X * X;
    YY := Y * Y;
    if XX + YY > 4.0 then
      Break;
    NewX := XX - YY + PxCx;
    Y := 2.0 * X * Y + PxCy;
    X := NewX;
    Inc(Iter);
  end;
  if Iter = MaxIter then
    CalcColor := 0
  else
    CalcColor := 1 + (((Iter * 12) + 24) mod 254);
end;

procedure Render(CoarseFirst: Boolean);
var
  T0, T1: LongInt;
  RowOff: Word;
  BR, BC, R2, C2, RMax, CMax: Integer;
  Interrupted: Boolean;
begin
  XMin := CenterX - SpanX * 0.5;
  XMax := CenterX + SpanX * 0.5;
  YMin := CenterY - SpanY * 0.5;
  YMax := CenterY + SpanY * 0.5;
  DX := (XMax - XMin) / Width;
  DY := (YMax - YMin) / Height;

  FrameCs := -1;
  DrawHud;
  T0 := NowCs;
  PendingAct := ActNone;
  Interrupted := False;

  if CoarseFirst then
  begin
    BR := 0;
    while BR < Height do
    begin
      Cy := YMin + BR * DY;
      RMax := BR + 2;
      if RMax >= Height then RMax := Height - 1;
      BC := 0;
      while BC < Width do
      begin
        if not (HasBox and (BC >= BoxX0) and (BC <= BoxX1) and
                (BR >= BoxY0) and (BR <= BoxY1)) then
        begin
          Cx := XMin + BC * DX;
          Color := CalcColor(Cx, Cy);
          CMax := BC + 2;
          if CMax >= Width then CMax := Width - 1;
          for R2 := BR to RMax do
          begin
            RowOff := Word(R2) * Width;
            for C2 := BC to CMax do
            begin
              if not (HasBox and (C2 >= BoxX0) and (C2 <= BoxX1) and
                      (R2 >= BoxY0) and (R2 <= BoxY1)) then
              begin
                if FrameBuf <> nil then
                  FrameBuf^[R2, C2] := Color;
                if R2 >= 9 then
                  Mem[$A000:RowOff + Word(C2)] := Color;
              end;
            end;
          end;
        end;
        if (MaxIter >= 64) and ((BC and 31) = 0) then
        begin
          if PollInput(PendingAct, PendingX, PendingY) then
          begin
            Interrupted := True;
            Break;
          end;
        end;
        Inc(BC, 3);
      end;
      if Interrupted then
        Break;
      if PollInput(PendingAct, PendingX, PendingY) then
      begin
        Interrupted := True;
        Break;
      end;
      Inc(BR, 3);
    end;
  end;

  if not Interrupted then
  begin
    for Row := 0 to Height - 1 do
    begin
      RowOff := Word(Row) * Width;
      Cy := YMin + Row * DY;
      for Col := 0 to Width - 1 do
      begin
        if not ((CoarseFirst and ((Row mod 3) = 0) and ((Col mod 3) = 0)) or
                (HasBox and (Col >= BoxX0) and (Col <= BoxX1) and
                 (Row >= BoxY0) and (Row <= BoxY1))) then
        begin
          Cx := XMin + Col * DX;
          Color := CalcColor(Cx, Cy);
          if Row >= 9 then
            Mem[$A000:RowOff + Word(Col)] := Color;
          if FrameBuf <> nil then
            FrameBuf^[Row, Col] := Color;
        end;
        if (MaxIter >= 64) and ((Col and 31) = 31) then
        begin
          if PollInput(PendingAct, PendingX, PendingY) then
          begin
            Interrupted := True;
            Break;
          end;
        end;
      end;
      if Interrupted then
        Break;
      if PollInput(PendingAct, PendingX, PendingY) then
      begin
        Interrupted := True;
        Break;
      end;
    end;
  end;

  if not Interrupted then
  begin
    T1 := NowCs;
    FrameCs := T1 - T0;
    if FrameCs < 0 then
      FrameCs := FrameCs + 8640000;
    HasBox := False;
    DrawHud;
  end;
end;

procedure ResetView;
begin
  CenterX := -0.6;
  CenterY := 0.0;
  SpanX := 3.2;
  SpanY := 2.4;
  ZoomLevel := 0;
  MaxIter := IterFor(0);
end;

begin
  FrameBuf := nil;
  TempBuf := nil;
  if MaxAvail >= SizeOf(TFrame) then
    New(FrameBuf);
  if MaxAvail >= SizeOf(TFrame) then
    New(TempBuf);
  if FrameBuf <> nil then
    FillChar(FrameBuf^, SizeOf(TFrame), 0);

  ResetView;
  HasBox := False;
  SetVideoMode($0013);
  SetPalette;
  MouseInit;
  ClearMousePresses;
  MouseShow;
  PendingAct := ActNone;
  Render(True);

  repeat
    if PendingAct = ActNone then
      if PollInput(PendingAct, PendingX, PendingY) then ;
    if PendingAct <> ActNone then
    begin
      Act := PendingAct;
      PendingAct := ActNone;
      case Act of
        ActZoomIn:
        begin
          PreviewZoom(True, PendingX, PendingY);
          CenterX := XMin + PendingX * DX;
          CenterY := YMin + PendingY * DY;
          SpanX := SpanX / 3.0;
          SpanY := SpanY / 3.0;
          Inc(ZoomLevel);
          MaxIter := IterFor(ZoomLevel);
          FrameCs := -1;
          DrawHud;
          Render(False);
        end;
        ActZoomOut:
        begin
          if SpanX < 25.0 then
          begin
            PreviewZoom(False, PendingX, PendingY);
            CenterX := XMin + PendingX * DX;
            CenterY := YMin + PendingY * DY;
            SpanX := SpanX * 3.0;
            SpanY := SpanY * 3.0;
            Dec(ZoomLevel);
            MaxIter := IterFor(ZoomLevel);
            FrameCs := -1;
            DrawHud;
            Render(True);
          end;
        end;
        ActReset:
        begin
          ResetView;
          HasBox := False;
          if FrameBuf <> nil then
            FillChar(FrameBuf^, SizeOf(TFrame), 0);
          FrameCs := -1;
          DrawHud;
          Render(True);
        end;
        ActQuit:
          Break;
      end;
    end;
  until False;

  MouseHide;
  SetVideoMode($0003);
  if TempBuf <> nil then
    Dispose(TempBuf);
  if FrameBuf <> nil then
    Dispose(FrameBuf);
end.
