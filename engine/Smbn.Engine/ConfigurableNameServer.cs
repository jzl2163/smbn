/*
 * Adapted from SMBLibrary.Server.NameServer by Tal Aloni.
 * This file is licensed under LGPL-3.0-or-later; see THIRD_PARTY_NOTICES.md.
 */
using System.Net;
using System.Net.Sockets;
using SMBLibrary.NetBios;

namespace Smbn.Engine;

internal sealed class ConfigurableNameServer : IDisposable
{
    private const int Port = 137;
    private readonly IPAddress _serverAddress;
    private readonly IPAddress _broadcastAddress;
    private readonly string _serverName;
    private readonly string _workgroup;
    private UdpClient? _client;
    private volatile bool _listening;

    internal ConfigurableNameServer(IPAddress serverAddress, IPAddress subnetMask, string serverName, string workgroup)
    {
        if (serverAddress.AddressFamily != AddressFamily.InterNetwork || IPAddress.Equals(serverAddress, IPAddress.Any))
        {
            throw new ArgumentException("NetBIOS name service requires a concrete IPv4 address.", nameof(serverAddress));
        }
        _serverAddress = serverAddress;
        _broadcastAddress = GetBroadcastAddress(serverAddress, subnetMask);
        _serverName = serverName;
        _workgroup = workgroup;
    }

    internal void Start()
    {
        if (_listening)
        {
            return;
        }
        _client = new UdpClient(new IPEndPoint(_serverAddress, Port));
        _client.EnableBroadcast = true;
        _listening = true;
        _client.BeginReceive(ReceiveCallback, null);
        Thread registrationThread = new(RegisterNames) { IsBackground = true, Name = "SMBN NBNS registration" };
        registrationThread.Start();
    }

    internal void Stop()
    {
        _listening = false;
        _client?.Dispose();
        _client = null;
    }

    private void ReceiveCallback(IAsyncResult result)
    {
        UdpClient? client = _client;
        if (!_listening || client is null)
        {
            return;
        }
        IPEndPoint? remote = null;
        byte[] buffer;
        try
        {
            buffer = client.EndReceive(result, ref remote);
        }
        catch (ObjectDisposedException) { return; }
        catch (SocketException) { return; }

        try
        {
            if (remote is not null && buffer.Length > NameServicePacketHeader.Length)
            {
                NameServicePacketHeader header = new(buffer, 0);
                if (header.OpCode == NameServiceOperation.QueryRequest)
                {
                    ProcessQuery(client, remote, buffer);
                }
            }
        }
        catch
        {
            // Malformed NBNS datagrams are ignored by design.
        }
        finally
        {
            try
            {
                if (_listening)
                {
                    client.BeginReceive(ReceiveCallback, null);
                }
            }
            catch (ObjectDisposedException) { }
            catch (SocketException) { }
        }
    }

    private void ProcessQuery(UdpClient client, IPEndPoint remote, byte[] buffer)
    {
        NameQueryRequest request = new(buffer, 0);
        if (request.Question.Type == NameRecordType.NB)
        {
            string requestedName = NetBiosUtils.GetNameFromMSNetBiosName(request.Question.Name);
            NetBiosSuffix suffix = (NetBiosSuffix)request.Question.Name[15];
            bool nameMatch = string.Equals(requestedName, _serverName, StringComparison.OrdinalIgnoreCase);
            if (nameMatch && (suffix == NetBiosSuffix.WorkstationService || suffix == NetBiosSuffix.FileServerService))
            {
                PositiveNameQueryResponse response = new();
                response.Header.TransactionID = request.Header.TransactionID;
                response.Resource.Name = request.Question.Name;
                response.Addresses.Add(_serverAddress.GetAddressBytes(), new NameFlags());
                byte[] bytes = response.GetBytes();
                client.Send(bytes, bytes.Length, remote);
            }
        }
        else
        {
            NodeStatusResponse response = new();
            response.Header.TransactionID = request.Header.TransactionID;
            response.Resource.Name = request.Question.Name;
            response.Names.Add(NetBiosUtils.GetMSNetBiosName(_serverName, NetBiosSuffix.WorkstationService), new NameFlags());
            response.Names.Add(NetBiosUtils.GetMSNetBiosName(_serverName, NetBiosSuffix.FileServerService), new NameFlags());
            NameFlags workgroupFlags = new() { WorkGroup = true };
            response.Names.Add(NetBiosUtils.GetMSNetBiosName(_workgroup, NetBiosSuffix.WorkstationService), workgroupFlags);
            byte[] bytes = response.GetBytes();
            client.Send(bytes, bytes.Length, remote);
        }
    }

    private void RegisterNames()
    {
        NameRegistrationRequest workstation = new(_serverName, NetBiosSuffix.WorkstationService, _serverAddress);
        NameRegistrationRequest fileServer = new(_serverName, NetBiosSuffix.FileServerService, _serverAddress);
        NameRegistrationRequest workgroup = new(_workgroup, NetBiosSuffix.WorkstationService, _serverAddress);
        workgroup.NameFlags.WorkGroup = true;
        RegisterName(workstation);
        RegisterName(fileServer);
        RegisterName(workgroup);
    }

    private void RegisterName(NameRegistrationRequest request)
    {
        byte[] packet = request.GetBytes();
        IPEndPoint broadcast = new(_broadcastAddress, Port);
        for (int index = 0; index < 4 && _listening; index++)
        {
            try { _client?.Send(packet, packet.Length, broadcast); }
            catch (ObjectDisposedException) { return; }
            catch (SocketException) { }
            if (index < 3) Thread.Sleep(250);
        }
    }

    private static IPAddress GetBroadcastAddress(IPAddress address, IPAddress subnetMask)
    {
        byte[] addressBytes = address.GetAddressBytes();
        byte[] maskBytes = subnetMask.GetAddressBytes();
        byte[] broadcast = new byte[addressBytes.Length];
        for (int index = 0; index < broadcast.Length; index++)
        {
            broadcast[index] = (byte)(addressBytes[index] | (maskBytes[index] ^ 255));
        }
        return new IPAddress(broadcast);
    }

    public void Dispose() => Stop();
}
