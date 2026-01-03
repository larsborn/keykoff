unit Main;

{$mode objfpc}{$H+}

interface

uses
  Classes, SysUtils, Forms, Controls, Graphics, Dialogs, StdCtrls, ExtCtrls,
  Menus, Windows, IniFiles;

type

  { TfMain }

  TfMain = class(TForm)
    btnSave: TButton;
    btnTest: TButton;
    edtCommand: TEdit;
    edtDirectory: TEdit;
    edtName: TEdit;
    edtParameters: TEdit;
    gbCommand: TGroupBox;
    Label1: TLabel;
    Label2: TLabel;
    Label3: TLabel;
    Label4: TLabel;
    lbCommands: TListBox;
    miNew: TMenuItem;
    miDelete: TMenuItem;
    OpenDialog1: TOpenDialog;
    pmList: TPopupMenu;
    SelectDirectoryDialog1: TSelectDirectoryDialog;
    procedure btnSaveClick(Sender: TObject);
    procedure btnTestClick(Sender: TObject);
    procedure FormCreate(Sender: TObject);
    procedure lbCommandsSelectionChange(Sender: TObject; User: boolean);
    procedure miDeleteClick(Sender: TObject);
    procedure miNewClick(Sender: TObject);
    procedure SetCreatingNew(Value: boolean);
  private
    CreatingNew: boolean;
  public

  end;

type TConfigRecord = class(TObject)
  Name: string;
  Executable: string;
  Parameters: string;
  Directory: string;
end;


var
  fMain: TfMain;

implementation

{$R *.lfm}

{ TfMain }

procedure ExtendWindowTitle(Caption: string);
var ColonPos : Integer;
begin
  ColonPos := Pos(':', fMain.Caption);
  if ColonPos <> 0 then
  begin
    fMain.Caption := Copy(fMain.Caption, 0, ColonPos - 1);
  end;
  if Caption <> '' then
  begin
    fMain.Caption := fMain.Caption + ': ' + Caption;
  end;
end;

function GetIniFileName: string;
begin
  GetIniFileName := 'QuickLauncher.ini';
end;

procedure ReadFromDisk();
var
  Config : TIniFile;
  Sections : TStringList;
  CurrentItem : TConfigRecord;
  i : integer;
begin
  Sections := TStringList.Create;
  Config := TIniFile.Create(GetIniFileName());
  Config.ReadSections(Sections);
  fMain.lbCommands.Clear;
  for i := 0 to Sections.Count - 1 do
  begin
    CurrentItem := TConfigRecord.Create;
    CurrentItem.Name := Sections[i];
    CurrentItem.Directory := Config.ReadString(Sections[i], 'Directory', '');
    CurrentItem.Executable := Config.ReadString(Sections[i], 'Executable', '');
    CurrentItem.Parameters := Config.ReadString(Sections[i], 'Parameters', '');
    fMain.lbCommands.AddItem(Sections[i], CurrentItem);
  end;
  Config.Free;
  Sections.Free;
end;

procedure WriteToDisk;
var
  Config : TIniFile;
  CurrentItem : TConfigRecord;
  i : integer;
begin
  DeleteFile(PChar(GetIniFileName()));
  Config := TIniFile.Create(GetIniFileName());
  for i := 0 to fMain.lbCommands.Count - 1 do
  begin
    CurrentItem := TConfigRecord(fMain.lbCommands.Items.Objects[i]);
    Config.WriteString(CurrentItem.Name, 'Executable', CurrentItem.Executable);
    Config.WriteString(CurrentItem.Name, 'Parameters', CurrentItem.Parameters);
    Config.WriteString(CurrentItem.Name, 'Directory', CurrentItem.Directory);
  end;
  Config.Free;
end;

procedure TfMain.SetCreatingNew(Value: boolean);
begin
  CreatingNew := Value;
  if Value then
    ExtendWindowTitle('New Command')
  else
    ExtendWindowTitle('');
end;

procedure TfMain.btnTestClick(Sender: TObject);
begin
  ShellExecute(
    handle,
    'open',
    PChar(edtCommand.Text),
    PChar(edtParameters.Text),
    PChar(edtDirectory.Text),
    1
  );
end;

procedure TfMain.FormCreate(Sender: TObject);
begin
  ReadFromDisk;
  lbCommandsSelectionChange(Sender, False);
end;

procedure TfMain.lbCommandsSelectionChange(Sender: TObject; User: boolean);
var   
  CurrentItem : TConfigRecord;
  i : integer;
  Found : boolean;
begin
  Found := False;
  for i := 0 to lbCommands.Count - 1 do
  begin
    if lbCommands.Selected[i] then
    begin
      CurrentItem := TConfigRecord(lbCommands.Items.Objects[i]);
      edtName.Text := CurrentItem.Name;
      edtDirectory.Text := CurrentItem.Directory;
      edtCommand.Text := CurrentItem.Executable;
      edtParameters.Text := CurrentItem.Parameters;
      Found := True;
      break;
    end;
  end;
  miDelete.Enabled := Found;
  gbCommand.Visible := Found or CreatingNew;
end;

procedure TfMain.miDeleteClick(Sender: TObject);
var
  i : integer;
begin
  for i := 0 to lbCommands.Count - 1 do
  begin
    if lbCommands.Selected[i] then
    begin
      lbCommands.Items.Delete(i);
      break;
    end;
  end;
  lbCommands.ClearSelection;
  lbCommandsSelectionChange(Sender, False);
  WriteToDisk;
end;

procedure TfMain.miNewClick(Sender: TObject);
begin
  SetCreatingNew(True);
  edtName.Text := '';
  edtCommand.Text := '';
  edtParameters.Text := '';
  edtDirectory.Text := '';
  lbCommands.ClearSelection;
  lbCommandsSelectionChange(Sender, False);
end;

procedure TfMain.btnSaveClick(Sender: TObject);
var
  CurrentItem : TConfigRecord;
  i : integer;
begin
  SetCreatingNew(False);
  i := lbCommands.Items.IndexOf(edtName.Text);
  if i = -1 then
    CurrentItem := TConfigRecord.Create
  else
    CurrentItem := TConfigRecord(lbCommands.Items.Objects[i]);

  CurrentItem.Name := edtName.Text;
  CurrentItem.Directory := edtDirectory.Text;
  CurrentItem.Executable := edtCommand.Text;
  CurrentItem.Parameters := edtParameters.Text;

  if i = -1 then
  begin
    lbCommands.AddItem(edtName.Text, CurrentItem);
    lbCommands.Selected[lbCommands.Count - 1] := true;
  end;

  WriteToDisk;
end;

end.

