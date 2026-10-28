// Isolated OS primitive only. No product Host, Commands, Engine or Codex server.
using System;
using System.ComponentModel;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Microsoft.Win32.SafeHandles;

namespace Drf04Peer {
    public sealed class Identity {
        public uint process_id;
        public uint process_api_id;
        public uint process_session_id;
        public uint token_session_id;
        public string user_sid;
        public string logon_sid;
        public bool queried_token_handle_closed;
        public bool queried_process_handle_closed;
    }
    public sealed class Peer {
        public uint pipe_process_id;
        public uint pipe_session_id;
        public string pid_api;
        public string session_api;
        public Identity token_identity;
    }
    [StructLayout(LayoutKind.Sequential)] struct SidAndAttributes { public IntPtr Sid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)] struct TokenGroupsHead { public uint Count; public SidAndAttributes First; }
    [StructLayout(LayoutKind.Sequential)] struct SecurityAttributes { public int Length; public IntPtr Descriptor; public int Inherit; }
    public sealed class OwnedServer {
        public IntPtr Handle;
        public NamedPipeServerStream Stream;
        public string Sddl;
        public bool Closed;
        public int AfterCloseError;
        public void Close() {
            if (Closed) return;
            Stream.Dispose(); // borrowed SafePipeHandle; raw ownership stays here
            AfterCloseError = Native.CloseChecked(Handle);
            Closed = true;
        }
    }
    public static class Native {
        [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] static extern IntPtr CreateNamedPipeW(string name,uint access,uint mode,uint instances,uint outSize,uint inSize,uint timeout,ref SecurityAttributes security);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetHandleInformation(IntPtr handle,out uint flags);
        [DllImport("kernel32.dll")] public static extern uint GetCurrentProcessId();
        [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access,bool inherit,uint pid);
        [DllImport("kernel32.dll", SetLastError=true)] static extern uint GetProcessId(IntPtr process);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool ProcessIdToSessionId(uint pid,out uint session);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetNamedPipeClientProcessId(IntPtr pipe,out uint pid);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetNamedPipeServerProcessId(IntPtr pipe,out uint pid);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetNamedPipeClientSessionId(IntPtr pipe,out uint session);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetNamedPipeServerSessionId(IntPtr pipe,out uint session);
        [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr memory);
        [DllImport("advapi32.dll", SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
        [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetTokenInformation(IntPtr token,int kind,IntPtr buffer,uint size,out uint needed);
        [DllImport("advapi32.dll", SetLastError=true, CharSet=CharSet.Unicode)] static extern bool ConvertSidToStringSidW(IntPtr sid,out IntPtr text);
        [DllImport("advapi32.dll", SetLastError=true, CharSet=CharSet.Unicode)] static extern bool ConvertStringSecurityDescriptorToSecurityDescriptorW(string sddl,uint version,out IntPtr descriptor,out uint size);
        static void Check(bool ok,string api) { if (!ok) throw new Win32Exception(Marshal.GetLastWin32Error(),api); }
        static string SidString(IntPtr sid) {
            IntPtr text; Check(ConvertSidToStringSidW(sid,out text),"ConvertSidToStringSidW");
            try { return Marshal.PtrToStringUni(text); } finally { LocalFree(text); }
        }
        static IntPtr TokenBuffer(IntPtr token,int kind) {
            uint needed; GetTokenInformation(token,kind,IntPtr.Zero,0,out needed);
            if (needed==0) throw new Win32Exception(Marshal.GetLastWin32Error(),"GetTokenInformation length");
            IntPtr buffer=Marshal.AllocHGlobal(checked((int)needed));
            if (!GetTokenInformation(token,kind,buffer,needed,out needed)) {
                int error=Marshal.GetLastWin32Error(); Marshal.FreeHGlobal(buffer);
                throw new Win32Exception(error,"GetTokenInformation data");
            }
            return buffer;
        }
        public static int ClosedError(IntPtr handle) {
            uint flags; if (GetHandleInformation(handle,out flags)) throw new InvalidOperationException("Owned handle still valid");
            int error=Marshal.GetLastWin32Error();
            if (error!=6) throw new Win32Exception(error,"GetHandleInformation after close");
            return error;
        }
        public static int CloseChecked(IntPtr handle) { Check(CloseHandle(handle),"CloseHandle"); return ClosedError(handle); }
        public static Identity ProcessIdentity(uint pid) {
            Identity result=new Identity { process_id=pid };
            IntPtr process=OpenProcess(0x1000,false,pid); if (process==IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(),"OpenProcess query peer");
            IntPtr token=IntPtr.Zero;
            try {
                result.process_api_id=GetProcessId(process); if (result.process_api_id==0) throw new Win32Exception(Marshal.GetLastWin32Error(),"GetProcessId");
                uint session; Check(ProcessIdToSessionId(pid,out session),"ProcessIdToSessionId"); result.process_session_id=session;
                Check(OpenProcessToken(process,8,out token),"OpenProcessToken actual peer");
                IntPtr user=TokenBuffer(token,1); try { result.user_sid=SidString(Marshal.ReadIntPtr(user)); } finally { Marshal.FreeHGlobal(user); }
                IntPtr groups=TokenBuffer(token,2);
                try {
                    uint count=unchecked((uint)Marshal.ReadInt32(groups));
                    int offset=Marshal.OffsetOf<TokenGroupsHead>("First").ToInt32();
                    int stride=Marshal.SizeOf<SidAndAttributes>();
                    for (uint i=0;i<count;i++) {
                        SidAndAttributes group=Marshal.PtrToStructure<SidAndAttributes>(IntPtr.Add(groups,checked(offset+(int)i*stride)));
                        if ((group.Attributes & 0xC0000000)==0xC0000000) { result.logon_sid=SidString(group.Sid); break; }
                    }
                    if (result.logon_sid==null) throw new InvalidOperationException("Actual token lacks logon SID");
                } finally { Marshal.FreeHGlobal(groups); }
                IntPtr ts=TokenBuffer(token,12); try { result.token_session_id=unchecked((uint)Marshal.ReadInt32(ts)); } finally { Marshal.FreeHGlobal(ts); }
            } finally {
                if (token!=IntPtr.Zero) { CloseChecked(token); result.queried_token_handle_closed=true; }
                CloseChecked(process); result.queried_process_handle_closed=true;
            }
            return result;
        }
        public static OwnedServer CreateServer(string shortName) {
            Identity current=ProcessIdentity(GetCurrentProcessId());
            string sddl="D:P(A;;GA;;;SY)(A;;GRGW;;;"+current.logon_sid+")";
            IntPtr sd; uint size; Check(ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl,1,out sd,out size),"ConvertStringSecurityDescriptor");
            IntPtr handle;
            try {
                SecurityAttributes sa=new SecurityAttributes { Length=Marshal.SizeOf<SecurityAttributes>(),Descriptor=sd,Inherit=0 };
                // DUPLEX + OVERLAPPED + FIRST_PIPE_INSTANCE; BYTE/WAIT + REJECT_REMOTE.
                handle=CreateNamedPipeW(@"\\.\pipe\"+shortName,3|0x40000000|0x00080000,8,1,65536,65536,0,ref sa);
                if (handle==new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error(),"CreateNamedPipeW actual server");
            } finally { LocalFree(sd); }
            return new OwnedServer { Handle=handle,Sddl=sddl,Stream=new NamedPipeServerStream(PipeDirection.InOut,true,false,new SafePipeHandle(handle,false)) };
        }
        public static Peer ObserveClient(IntPtr pipe) {
            uint pid,session; Check(GetNamedPipeClientProcessId(pipe,out pid),"GetNamedPipeClientProcessId");
            Check(GetNamedPipeClientSessionId(pipe,out session),"GetNamedPipeClientSessionId");
            return new Peer { pipe_process_id=pid,pipe_session_id=session,pid_api="GetNamedPipeClientProcessId",session_api="GetNamedPipeClientSessionId",token_identity=ProcessIdentity(pid) };
        }
        public static Peer ObserveServer(IntPtr pipe) {
            uint pid,session; Check(GetNamedPipeServerProcessId(pipe,out pid),"GetNamedPipeServerProcessId");
            Check(GetNamedPipeServerSessionId(pipe,out session),"GetNamedPipeServerSessionId");
            return new Peer { pipe_process_id=pid,pipe_session_id=session,pid_api="GetNamedPipeServerProcessId",session_api="GetNamedPipeServerSessionId",token_identity=ProcessIdentity(pid) };
        }
        public static void Wait(Task task) { if (!task.Wait(10000)) throw new TimeoutException("Owned pipe fixture IO/connection exceeded 10s"); task.GetAwaiter().GetResult(); }
        static async Task<string> ReadFrameAsync(Stream stream) {
            byte[] header=new byte[4]; int n=0;
            while(n<4) { int got=await stream.ReadAsync(header,n,4-n); if(got==0) { if(n==0)return null; throw new EndOfStreamException("truncated header"); } n+=got; }
            int size=BitConverter.ToInt32(header,0); if(size<0||size>1048576)throw new InvalidDataException("fixture IPC frame size");
            byte[] data=new byte[size]; n=0;
            while(n<size) { int got=await stream.ReadAsync(data,n,size-n); if(got==0)throw new EndOfStreamException("truncated body");n+=got; }
            return new UTF8Encoding(false,true).GetString(data);
        }
        public static string ReadFrame(Stream stream) { Task<string> task=ReadFrameAsync(stream); Wait(task); return task.Result; }
        public static void WriteFrame(Stream stream,string text) {
            byte[] data=Encoding.UTF8.GetBytes(text); byte[] header=BitConverter.GetBytes(data.Length);
            Wait(stream.WriteAsync(header,0,4)); Wait(stream.WriteAsync(data,0,data.Length)); Wait(stream.FlushAsync());
        }
    }
}
