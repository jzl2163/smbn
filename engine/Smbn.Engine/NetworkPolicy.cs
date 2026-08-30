using System.Net;
using System.Net.Sockets;

namespace Smbn.Engine;

internal sealed class NetworkPolicy
{
    private readonly List<IpNetwork> _allow;
    private readonly List<IpNetwork> _reject;

    internal NetworkPolicy(IEnumerable<string> allow, IEnumerable<string> reject)
    {
        _allow = allow.Where(static x => !string.IsNullOrWhiteSpace(x)).Select(IpNetwork.Parse).ToList();
        _reject = reject.Where(static x => !string.IsNullOrWhiteSpace(x)).Select(IpNetwork.Parse).ToList();
    }

    internal bool Allows(IPAddress address)
    {
        address = NetworkPolicy.Normalize(address);
        if (_reject.Any(network => network.Contains(address)))
        {
            return false;
        }
        return _allow.Count == 0 || _allow.Any(network => network.Contains(address));
    }

    private static IPAddress Normalize(IPAddress address) =>
        address.IsIPv4MappedToIPv6 ? address.MapToIPv4() : address;

    private sealed class IpNetwork
    {
        private readonly byte[] _networkBytes;
        private readonly int _prefixLength;
        private readonly AddressFamily _family;

        private IpNetwork(IPAddress network, int prefixLength)
        {
            network = NetworkPolicy.Normalize(network);
            _networkBytes = network.GetAddressBytes();
            _prefixLength = prefixLength;
            _family = network.AddressFamily;
        }

        internal static IpNetwork Parse(string value)
        {
            string[] parts = value.Trim().Split('/', 2, StringSplitOptions.TrimEntries);
            if (!IPAddress.TryParse(parts[0], out IPAddress? address))
            {
                throw new ArgumentException($"Invalid CIDR address: {value}");
            }
            address = NetworkPolicy.Normalize(address);
            int bitCount = address.AddressFamily == AddressFamily.InterNetwork ? 32 : 128;
            int prefix = parts.Length == 1 ? bitCount :
                int.TryParse(parts[1], out int parsed) ? parsed : -1;
            if (prefix < 0 || prefix > bitCount)
            {
                throw new ArgumentException($"Invalid CIDR prefix: {value}");
            }
            return new IpNetwork(address, prefix);
        }

        internal bool Contains(IPAddress address)
        {
            address = NetworkPolicy.Normalize(address);
            if (address.AddressFamily != _family)
            {
                return false;
            }

            byte[] candidate = address.GetAddressBytes();
            int fullBytes = _prefixLength / 8;
            int remainingBits = _prefixLength % 8;
            for (int index = 0; index < fullBytes; index++)
            {
                if (_networkBytes[index] != candidate[index])
                {
                    return false;
                }
            }
            if (remainingBits == 0)
            {
                return true;
            }
            int mask = 0xff << (8 - remainingBits);
            return (_networkBytes[fullBytes] & mask) == (candidate[fullBytes] & mask);
        }
    }
}
